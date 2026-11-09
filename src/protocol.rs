//! Wire format for the K617 vendor HID interface.
//!
//! Everything here was recovered from `KeyboardDrv.exe` (the 1.6.6 Windows
//! app) and verified against real hardware. The device exposes three useful
//! feature reports on interface 1 (usage page 0xFF00, usage 0x0001):
//!
//! * report 0x05, 6 bytes  - handshake / short command channel
//! * report 0x06, 1032 bytes - 8-byte block header + 1024-byte payload
//! * report 0x08, 382 bytes - direct per-key LED frame (Sinodragon)
//!
//! The Windows app builds each 1032-byte block with one of four opcodes:
//!
//! ```text
//! 06 08 b8 00 40 00 00 00   profile/effect data (1003 input bytes at +0x15)
//! 06 09 xx 00 40 00 00 00   LED matrix write, xx = cursor (0x00/0xbc/0xc0)
//! 06 05 xx 00 40 00 00 00   key remap matrix write, xx = cursor
//! 06 04 d4 00 40 00 00 00   settings matrix
//! 06 03 b6 00 00 00 00 00   commit: 136-byte config at +8
//! ```
//!
//! An effect change is a five-frame burst: handshake, profile block, two LED
//! matrix blocks, then the commit block. The commit block's byte 21 is the
//! firmware effect index, bytes 29..31 of the profile block are the base
//! colour, and bytes 69/71 of the commit block pack speed/brightness as
//! nibbles (high nibble speed, low nibble brightness).

use crate::effects::Effect;
use crate::keymap::KeyDef;

pub const CMD_REPORT: u8 = 0x05;
pub const DATA_REPORT: u8 = 0x06;
pub const PERKEY_REPORT: u8 = 0x08;

pub const DATA_FRAME_LEN: usize = 1032;
pub const PERKEY_FRAME_LEN: usize = 382;

/// The captured frames use the same hardware defaults as the Windows app.
/// They are stored as raw bytes so the exact padding that firmware expects is
/// preserved. Only the documented parameter bytes are patched at runtime.
const TEMPLATE_HANDSHAKE: &[u8; 6] = include_bytes!("../assets/effect_0.bin");
const TEMPLATE_PROFILE: &[u8; DATA_FRAME_LEN] = include_bytes!("../assets/effect_1.bin");
const TEMPLATE_MATRIX_A: &[u8; DATA_FRAME_LEN] = include_bytes!("../assets/effect_2.bin");
const TEMPLATE_MATRIX_B: &[u8; DATA_FRAME_LEN] = include_bytes!("../assets/effect_3.bin");
const TEMPLATE_COMMIT: &[u8; DATA_FRAME_LEN] = include_bytes!("../assets/effect_4.bin");

/// Offset of the base colour inside the profile block frame.
const PROFILE_COLOR: usize = 29;
/// Offset of the effect index inside the commit frame.
const COMMIT_MODE: usize = 21;
/// The vendor app stores the packed speed/brightness value twice.
const COMMIT_SPEED_BRIGHT: [usize; 2] = [69, 71];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
  pub r: u8,
  pub g: u8,
  pub b: u8,
}

impl Rgb {
  pub const OFF: Rgb = Rgb { r: 0, g: 0, b: 0 };

  pub fn new(r: u8, g: u8, b: u8) -> Self {
    Self { r, g, b }
  }

  /// Accepts `RRGGBB`, `#RRGGBB`, `0xRRGGBB`, `r,g,b` and the handful of
  /// names that were present in the vendor UI.
  pub fn parse(input: &str) -> Option<Self> {
    let text = input.trim();
    if let Some((r, g, b)) = parse_triplet(text) {
      return Some(Rgb::new(r, g, b));
    }
    let named = match text.to_ascii_lowercase().as_str() {
      "red" => (255, 0, 0),
      "green" => (0, 255, 0),
      "blue" => (0, 0, 255),
      "white" => (255, 255, 255),
      "black" | "off" => (0, 0, 0),
      "yellow" => (255, 255, 0),
      "cyan" => (0, 255, 255),
      "magenta" | "purple" => (255, 0, 255),
      "orange" => (255, 128, 0),
      _ => return None,
    };
    Some(Rgb::new(named.0, named.1, named.2))
  }

  pub fn to_hex(self) -> String {
    format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
  }
}

fn parse_triplet(text: &str) -> Option<(u8, u8, u8)> {
  if text.contains(',') {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 3 {
      return None;
    }
    let values: Vec<u8> = parts
      .iter()
      .map(|p| p.trim().parse::<u8>())
      .collect::<Result<_, _>>()
      .ok()?;
    return Some((values[0], values[1], values[2]));
  }
  let hex = text
    .strip_prefix('#')
    .or_else(|| text.strip_prefix("0x"))
    .or_else(|| text.strip_prefix("0X"))
    .unwrap_or(text);
  if hex.len() != 6 || !hex.is_ascii() {
    return None;
  }
  let value = u32::from_str_radix(hex, 16).ok()?;
  Some(((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

#[derive(Clone, Copy, Debug)]
pub struct EffectParams {
  pub effect: Effect,
  pub color: Rgb,
  /// 1..=4 in the vendor UI. The protocol nibble itself is 0..=15.
  pub speed: u8,
  pub brightness: u8,
}

impl EffectParams {
  pub fn packed_speed_brightness(&self) -> u8 {
    ((self.speed & 0x0f) << 4) | (self.brightness & 0x0f)
  }
}

/// Build the five feature-report frames that switch firmware effects.
///
/// The keyboard renders the effect itself; we only send the mode, base
/// colour and speed/brightness. The remaining bytes are the vendor defaults
/// captured from the official software and are required to keep the firmware
/// parser happy.
pub fn effect_burst(params: EffectParams) -> Vec<Vec<u8>> {
  let mut profile = *TEMPLATE_PROFILE;
  let mut commit = *TEMPLATE_COMMIT;

  profile[PROFILE_COLOR] = params.color.r;
  profile[PROFILE_COLOR + 1] = params.color.g;
  profile[PROFILE_COLOR + 2] = params.color.b;

  commit[COMMIT_MODE] = params.effect.number();
  let packed = params.packed_speed_brightness();
  for offset in COMMIT_SPEED_BRIGHT {
    commit[offset] = packed;
  }

  vec![
    TEMPLATE_HANDSHAKE.to_vec(),
    profile.to_vec(),
    TEMPLATE_MATRIX_A.to_vec(),
    TEMPLATE_MATRIX_B.to_vec(),
    commit.to_vec(),
  ]
}

/// The handshake the Windows app sends before reading the committed state.
pub fn read_state_request() -> [u8; 6] {
  [0x05, 0x83, 0xb6, 0x00, 0x00, 0x00]
}

/// Short command that asks the keyboard for its six-byte device password.
pub fn password_request() -> [u8; 6] {
  [0x05, 0x81, 0x00, 0x00, 0x00, 0x00]
}

/// Extract the password from a report-5 response. Layout:
/// `05 01 p0 p1 p2 p3 p4 p5` (the 8-byte read is needed to get all six).
pub fn parse_password(response: &[u8]) -> Option<[u8; 6]> {
  if response.len() < 8 || response[0] != CMD_REPORT || response[1] != 0x01 {
    return None;
  }
  let mut password = [0u8; 6];
  password.copy_from_slice(&response[2..8]);
  Some(password)
}

/// Offset of the committed effect index in a report-6 state read.
pub const STATE_MODE: usize = COMMIT_MODE;
/// Offsets of the committed speed/brightness pair in a state read.
pub const STATE_SPEED_BRIGHT: [usize; 2] = COMMIT_SPEED_BRIGHT;

/// Build a direct per-key frame for report 8.
///
/// The Sinodragon frame carries 96 RGB slots (16 columns x 6 rows) in
/// column-major order. Slots that do not exist on the K617 are left black.
pub fn per_key_frame(colors: &[(KeyDef, Rgb)]) -> Vec<u8> {
  let mut frame = vec![0u8; PERKEY_FRAME_LEN];
  frame[0] = PERKEY_REPORT;
  frame[1..4].copy_from_slice(&[0x0a, 0x7a, 0x01]);
  for (key, color) in colors {
    let slot = key.sinodragon_slot as usize;
    if slot >= 96 {
      continue;
    }
    let offset = 4 + slot * 3;
    frame[offset] = color.r;
    frame[offset + 1] = color.g;
    frame[offset + 2] = color.b;
  }
  frame
}

/// A solid colour across all 61 keys.
pub fn solid_frame(color: Rgb) -> Vec<u8> {
  let colors: Vec<(KeyDef, Rgb)> = crate::keymap::KEYS.iter().map(|k| (*k, color)).collect();
  per_key_frame(&colors)
}

/// Build a per-key frame from `led_index -> colour` overrides, filling every
/// other key with `base`.
pub fn per_key_frame_with_base(base: Rgb, overrides: &[(usize, Rgb)]) -> Vec<u8> {
  let mut frame = vec![0u8; PERKEY_FRAME_LEN];
  frame[0] = PERKEY_REPORT;
  frame[1..4].copy_from_slice(&[0x0a, 0x7a, 0x01]);
  for key in crate::keymap::KEYS.iter() {
    let color = overrides
      .iter()
      .find(|(index, _)| *index == key.led_index)
      .map(|(_, color)| *color)
      .unwrap_or(base);
    let slot = key.sinodragon_slot as usize;
    if slot >= 96 {
      continue;
    }
    let offset = 4 + slot * 3;
    frame[offset] = color.r;
    frame[offset + 1] = color.g;
    frame[offset + 2] = color.b;
  }
  frame
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_colors() {
    assert_eq!(Rgb::parse("#ff8000"), Some(Rgb::new(255, 128, 0)));
    assert_eq!(Rgb::parse("00FF00"), Some(Rgb::new(0, 255, 0)));
    assert_eq!(Rgb::parse("1,2,3"), Some(Rgb::new(1, 2, 3)));
    assert_eq!(Rgb::parse("nope"), None);
  }

  #[test]
  fn burst_patches_only_documented_fields() {
    let frames = effect_burst(EffectParams {
      effect: Effect::SineWave,
      color: Rgb::new(1, 2, 3),
      speed: 4,
      brightness: 2,
    });
    assert_eq!(frames.len(), 5);
    assert_eq!(&frames[1][29..32], &[1, 2, 3]);
    assert_eq!(frames[4][21], 13);
    assert_eq!(frames[4][69], 0x42);
    assert_eq!(frames[4][71], 0x42);
  }

  #[test]
  fn template_headers_match_the_documented_opcodes() {
    let frames = effect_burst(EffectParams {
      effect: Effect::FixedOn,
      color: Rgb::new(0, 0, 0),
      speed: 3,
      brightness: 3,
    });
    assert_eq!(&frames[0][..], &[5, 0x83, 0xb6, 0, 0, 0]);
    assert_eq!(&frames[1][..8], &[6, 0x08, 0xb8, 0, 0x40, 0, 0, 0]);
    assert_eq!(&frames[2][..8], &[6, 0x09, 0xbc, 0, 0x40, 0, 0, 0]);
    assert_eq!(&frames[3][..8], &[6, 0x09, 0xc0, 0, 0x40, 0, 0, 0]);
    assert_eq!(&frames[4][..8], &[6, 0x03, 0xb6, 0, 0, 0, 0, 0]);
    assert_eq!(
      frames.iter().map(|f| f.len()).collect::<Vec<_>>(),
      vec![6, 1032, 1032, 1032, 1032]
    );
  }

  #[test]
  fn per_key_frame_places_slots() {
    let frame = solid_frame(Rgb::new(9, 8, 7));
    assert_eq!(frame.len(), 382);
    assert_eq!(&frame[..4], &[8, 0x0a, 0x7a, 0x01]);
    // Escape is Sinodragon slot 0, so it is the first triplet.
    assert_eq!(&frame[4..7], &[9, 8, 7]);
    // Space is slot 35.
    assert_eq!(&frame[4 + 35 * 3..4 + 35 * 3 + 3], &[9, 8, 7]);
  }

  #[test]
  fn password_layout() {
    let mut response = [0u8; 8];
    response[0] = 5;
    response[1] = 1;
    response[2..8].copy_from_slice(&[2, 0, 0, 0, 0, 0x1a]);
    assert_eq!(parse_password(&response), Some([2, 0, 0, 0, 0, 0x1a]));
  }

  #[test]
  fn key_count_is_61() {
    assert_eq!(crate::keymap::KEY_COUNT, 61);
  }
}
