//! Host-side pulsing red wave effect.
//!
//! This is a monochrome red rolling wave with two humps. It continuously
//! morphs between the two-hump M (humps up, centre and ends down) and its
//! inverse, passing through a single-arch transition. The quadrature term
//! keeps the shape moving and curved: it never pauses at the extremes and
//! never collapses to a straight line. The band is thick, spans the full
//! keyboard, and every key outside it stays black.
//!
//! The geometry uses the real key centres, so wide keys such as Space are
//! included. Only the keys inside the band are non-black.

use std::f32::consts::TAU;

use crate::keymap::{KeyDef, CENTERS, KEYS};
use crate::protocol::Rgb;

/// Vertical centre of the wave, 0 = top of the key area, 1 = bottom.
const CENTRE: f32 = 0.50;
/// How far the humps swing above and below `CENTRE`.
const AMPLITUDE: f32 = 0.38;
/// Strength of the single-arch quadrature component. This is what keeps the
/// transition curved and moving instead of pausing on a flat line.
const QUADRATURE: f32 = 0.45;
/// Base half-thickness as a fraction of the key-area height. The band is
/// deliberately fat: about two visible key rows, not a single row.
const THICKNESS: f32 = 0.26;
/// Bounds of the actual key centres inside the 560x212 preview image.
/// Using the leftmost and rightmost key centres (Escape and RCtrl) keeps the
/// pulse symmetric and makes the far-right bottom key part of the wave.
const X_MIN: f32 = 25.0;
const X_MAX: f32 = 530.5;
const Y_MIN: f32 = 14.0;
const Y_MAX: f32 = 188.0;

/// One animation frame. Returns one colour per K617 key. Unlit keys are black.
pub fn frame(phase: f32) -> Vec<(KeyDef, Rgb)> {
  KEYS
    .iter()
    .enumerate()
    .map(|(index, key)| {
      // Use the real key centres from Cfg.ini so wide keys such as Space are
      // included and the wave spans the whole physical keyboard.
      let (cx, cy) = CENTERS[index];
      let x = ((cx - X_MIN) / (X_MAX - X_MIN)).clamp(0.0, 1.0);
      let y = ((cy - Y_MIN) / (Y_MAX - Y_MIN)).clamp(0.0, 1.0);
      // cos(4*pi*x) is +1 at the centre and the two ends, -1 at the two
      // inner humps. The sin term pulses the M shape up and down. The cos
      // term adds a single arch in quadrature, so when the M amplitude is
      // zero the band is still curved and still moving.
      let m_shape = (2.0 * TAU * x).cos();
      let arch = (TAU * x).cos();
      let centre = CENTRE + AMPLITUDE * (m_shape * phase.sin() + QUADRATURE * arch * phase.cos());
      let distance = (y - centre).abs();

      // Two low-frequency terms give a slightly ragged edge. The amplitude
      // is deliberately small: enough to read as distressed, not noisy.
      let thickness = THICKNESS
        + 0.022 * (cx * 0.013 + phase * 1.7).sin()
        + 0.012 * (cx * 0.006 - cy * 0.009 + phase * 2.1).sin();
      if distance >= thickness {
        return (*key, Rgb::OFF);
      }

      // Edge keys stay clearly red instead of fading to black. That is what
      // makes the band read as thick on hardware: a pure linear falloff only
      // leaves the centre row visibly bright.
      let falloff = (1.0 - distance / thickness).powf(0.65);
      let intensity = 0.42 + 0.58 * falloff;
      // Keep the core bright red and fade the border through red. The
      // green/blue crumbs only appear at the very centre so it still looks red.
      let r = (42.0 + 213.0 * intensity).round() as u8;
      let g = (18.0 * intensity.powi(3)).round() as u8;
      let b = (12.0 * intensity.powi(3)).round() as u8;
      (*key, Rgb::new(r, g, b))
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn lit_names(phase: f32) -> Vec<&'static str> {
    frame(phase)
      .into_iter()
      .filter(|(_, color)| *color != Rgb::OFF)
      .map(|(key, _)| key.name)
      .collect()
  }

  #[test]
  fn lights_a_band_and_leaves_gaps() {
    let lit = lit_names(0.0);
    assert!(lit.len() > 12, "wave should light a thick band: {lit:?}");
    assert!(lit.len() < KEYS.len(), "some keys must stay off: {lit:?}");
  }

  #[test]
  fn wave_moves_with_phase() {
    let a = lit_names(std::f32::consts::FRAC_PI_2);
    let b = lit_names(-std::f32::consts::FRAC_PI_2);
    assert_ne!(a, b);
  }

  #[test]
  fn covers_the_full_keyboard_including_space() {
    let mut saw_space = false;
    let mut saw_top = false;
    let mut saw_lctrl = false;
    let mut saw_rctrl = false;
    for step in 0..120 {
      let phase = step as f32 * TAU / 120.0;
      for (key, color) in frame(phase) {
        if color == Rgb::OFF {
          continue;
        }
        match key.name {
          "Space" => saw_space = true,
          "Escape" => saw_top = true,
          "LCtrl" => saw_lctrl = true,
          "RCtrl" => saw_rctrl = true,
          _ => {}
        }
      }
    }
    assert!(saw_space, "spacebar must be part of the wave");
    assert!(saw_top, "top row must be part of the wave");
    assert!(saw_lctrl, "left Ctrl must be part of the wave");
    assert!(saw_rctrl, "right Ctrl must be part of the wave");
  }

  #[test]
  fn right_side_uses_vendor_slots() {
    assert_eq!(crate::keymap::find("Menu").unwrap().sinodragon_slot, 77);
    assert_eq!(crate::keymap::find("RCtrl").unwrap().sinodragon_slot, 83);
  }

  #[test]
  fn two_humps_alternate_with_centre() {
    fn average_y(phase: f32, lo: f32, hi: f32) -> f32 {
      let mut sum = 0.0;
      let mut count = 0.0f32;
      for (index, (_, color)) in frame(phase).iter().enumerate() {
        if *color == Rgb::OFF {
          continue;
        }
        let x = ((CENTERS[index].0 - X_MIN) / (X_MAX - X_MIN)).clamp(0.0, 1.0);
        if (lo..=hi).contains(&x) {
          sum += CENTERS[index].1;
          count += 1.0;
        }
      }
      sum / count.max(1.0)
    }

    let hump_up = average_y(std::f32::consts::FRAC_PI_2, 0.15, 0.35);
    let centre_down = average_y(std::f32::consts::FRAC_PI_2, 0.45, 0.55);
    assert!(
      hump_up + 20.0 < centre_down,
      "humps should sit above the centre valley: {hump_up} vs {centre_down}"
    );

    let hump_down = average_y(-std::f32::consts::FRAC_PI_2, 0.15, 0.35);
    let centre_up = average_y(-std::f32::consts::FRAC_PI_2, 0.45, 0.55);
    assert!(
      hump_down > centre_up + 20.0,
      "inverse phase should flip the shape: {hump_down} vs {centre_up}"
    );
  }

  #[test]
  fn transition_is_never_flat() {
    fn average_y(phase: f32, lo: f32, hi: f32) -> f32 {
      let mut sum = 0.0;
      let mut count = 0.0f32;
      for (index, (_, color)) in frame(phase).iter().enumerate() {
        if *color == Rgb::OFF {
          continue;
        }
        let x = ((CENTERS[index].0 - X_MIN) / (X_MAX - X_MIN)).clamp(0.0, 1.0);
        if (lo..=hi).contains(&x) {
          sum += CENTERS[index].1;
          count += 1.0;
        }
      }
      sum / count.max(1.0)
    }
    let centre = average_y(0.0, 0.45, 0.55);
    let hump = average_y(0.0, 0.20, 0.30);
    assert!(
      (centre - hump).abs() > 10.0,
      "transition must stay curved, got centre={centre} hump={hump}"
    );
  }

  #[test]
  fn colours_stay_red() {
    for (_, color) in frame(0.4) {
      assert!(color.r >= color.g && color.r >= color.b);
    }
  }
}
