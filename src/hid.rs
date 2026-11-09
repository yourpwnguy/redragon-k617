//! HID transport for the K617.
//!
//! We deliberately speak the kernel hidraw interface through hidapi instead of
//! writing a custom USB driver. That matches what the Windows app does with
//! HidD_SetFeature/HidD_GetFeature and keeps the port dependency-free at the
//! kernel level.
//!
//! Two details matter for correctness:
//!
//! * The vendor control interface is interface 1 (usage page 0xFF00, usage 1).
//!   Interface 0 is the boot keyboard and does not accept feature reports.
//! * The official app passes oversized buffers to HidD_SetFeature. Windows
//!   clamps them to the report length, but hidraw rejects a length that does
//!   not match the report descriptor, so every frame here is exactly sized
//!   (6 bytes for report 5, 1032 for report 6, 382 for report 8).

use std::thread::sleep;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use hidapi::{HidApi, HidDevice};

use crate::protocol::{self, parse_password, EffectParams, DATA_FRAME_LEN, PERKEY_FRAME_LEN};

pub const VENDOR_ID: u16 = 0x258a;
pub const PRODUCT_ID: u16 = 0x0049;
pub const VENDOR_INTERFACE: i32 = 1;

pub struct Keyboard {
  device: HidDevice,
  path: String,
  interface: i32,
}

impl Keyboard {
  /// Open the vendor control interface. If several matching devices are
  /// present (for example two keyboards) the first interface 1 wins.
  pub fn open() -> Result<Self> {
    let api = HidApi::new().context("initialising hidapi")?;
    let mut fallback: Option<(String, i32)> = None;
    let mut chosen: Option<(String, i32)> = None;
    for info in api.device_list() {
      if info.vendor_id() != VENDOR_ID || info.product_id() != PRODUCT_ID {
        continue;
      }
      let interface = info.interface_number();
      let path = info.path().to_string_lossy().into_owned();
      if interface == VENDOR_INTERFACE {
        chosen = Some((path, interface));
        break;
      }
      if fallback.is_none() {
        fallback = Some((path, interface));
      }
    }
    let (path, interface) = chosen.or(fallback).ok_or_else(|| {
      anyhow!(
        "Redragon K617 ({VENDOR_ID:04x}:{PRODUCT_ID:04x}) not found. \
                     Check the cable and udev permissions."
      )
    })?;

    let device = api
      .open_path(std::ffi::CString::new(path.as_str())?.as_c_str())
      .with_context(|| format!("opening {path} (interface {interface})"))?;
    Ok(Self {
      device,
      path,
      interface,
    })
  }

  pub fn path(&self) -> &str {
    &self.path
  }

  pub fn interface(&self) -> i32 {
    self.interface
  }

  /// Send one feature report, retrying a couple of times because some hubs
  /// briefly stall the control endpoint right after enumeration.
  pub fn send(&self, frame: &[u8]) -> Result<()> {
    let mut last_error = None;
    for attempt in 0..3 {
      match self.device.send_feature_report(frame) {
        Ok(()) => return Ok(()),
        Err(error) => {
          last_error = Some(error);
          if attempt < 2 {
            sleep(Duration::from_millis(10));
          }
        }
      }
    }
    Err(
      last_error
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow!("feature report failed"))
        .context(format!("sending {} byte feature report", frame.len())),
    )
  }

  pub fn send_burst(&self, frames: &[Vec<u8>]) -> Result<()> {
    for frame in frames {
      self.send(frame)?;
      sleep(Duration::from_millis(10));
    }
    Ok(())
  }

  /// Ask for the six-byte device password. The command is a report-5 write
  /// followed by an 8-byte report-5 read; the extra two bytes are needed
  /// because the password lives at offsets 2..8 of the response.
  pub fn password(&self) -> Result<[u8; 6]> {
    self.send(&protocol::password_request())?;
    sleep(Duration::from_millis(20));
    let mut buffer = vec![0u8; 8];
    buffer[0] = protocol::CMD_REPORT;
    let read = self
      .device
      .get_feature_report(&mut buffer)
      .context("reading device password")?;
    parse_password(&buffer[..read])
      .ok_or_else(|| anyhow!("unexpected password response: {:02x?}", &buffer[..read]))
  }

  /// Read the committed 1032-byte configuration block. The device only
  /// answers report-6 GET requests after the `83 b6` handshake.
  pub fn read_state(&self) -> Result<Vec<u8>> {
    self.send(&protocol::read_state_request())?;
    sleep(Duration::from_millis(20));
    let mut buffer = vec![0u8; DATA_FRAME_LEN];
    buffer[0] = protocol::DATA_REPORT;
    let read = self
      .device
      .get_feature_report(&mut buffer)
      .context("reading committed state")?;
    buffer.truncate(read);
    Ok(buffer)
  }

  /// Apply an effect and verify that the keyboard committed the mode byte.
  /// Verification is best effort: some firmware revisions answer the state
  /// read slowly, but the effect itself is already applied at that point.
  pub fn apply_effect(&self, params: EffectParams) -> Result<()> {
    let frames = protocol::effect_burst(params);
    self.send_burst(&frames)?;
    sleep(Duration::from_millis(30));
    match self.read_state() {
      Ok(state)
        if state.len() > protocol::STATE_MODE
          && state[protocol::STATE_MODE] != params.effect.number() =>
      {
        // One retry is enough; a second failure means the frame was
        // accepted but the firmware is in a different mode.
        self.send_burst(&frames)?;
      }
      _ => {}
    }
    Ok(())
  }

  pub fn send_per_key(&self, frame: &[u8]) -> Result<()> {
    if frame.len() != PERKEY_FRAME_LEN {
      return Err(anyhow!(
        "per-key frame must be {PERKEY_FRAME_LEN} bytes, got {}",
        frame.len()
      ));
    }
    self.send(frame)
  }
}
