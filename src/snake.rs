//! Host-side thick S-wave snake effect.
//!
//! This is a monochrome red wave that spans the keyboard horizontally. The
//! body is a thick band around a sine centreline, with a small amount of
//! smooth thickness variation so the edges look distressed rather than
//! laser-cut. The wave travels by shifting the phase, which is what makes it
//! move like a snake.
//!
//! The geometry uses the Sinodragon matrix slots (16 columns x 6 rows) rather
//! than pixel coordinates, so the same frame works for the CLI streamer and a
//! future GUI preview. Only the keys inside the band are non-black; every
//! other key is switched off.

use std::f32::consts::TAU;

use crate::keymap::{KeyDef, CENTERS, KEYS};
use crate::protocol::Rgb;

/// Vertical centre of the wave, 0 = top of the key area, 1 = bottom.
const CENTRE: f32 = 0.50;
/// How far the centreline swings above and below `CENTRE`.
const AMPLITUDE: f32 = 0.30;
/// Number of full S curves across the used keyboard width.
const PERIODS: f32 = 1.5;
/// Base half-thickness as a fraction of the key-area height. The band is
/// deliberately fat: about two and a half visible key rows, not a single row.
const THICKNESS: f32 = 0.24;
/// Bounds of the key area inside the 560x212 preview image.
const X_MIN: f32 = 13.0;
const X_MAX: f32 = 547.0;
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
      let centre = CENTRE + AMPLITUDE * (TAU * PERIODS * x - phase).sin();
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
    let a = lit_names(0.0);
    let b = lit_names(std::f32::consts::PI);
    assert_ne!(a, b);
  }

  #[test]
  fn covers_the_full_keyboard_including_space() {
    let mut saw_space = false;
    let mut saw_top = false;
    let mut saw_bottom = false;
    for step in 0..120 {
      let phase = step as f32 * TAU / 120.0;
      for (key, color) in frame(phase) {
        if color == Rgb::OFF {
          continue;
        }
        match key.name {
          "Space" => saw_space = true,
          "Escape" => saw_top = true,
          "LCtrl" | "RCtrl" => saw_bottom = true,
          _ => {}
        }
      }
    }
    assert!(saw_space, "spacebar must be part of the wave");
    assert!(saw_top, "top row must be part of the wave");
    assert!(saw_bottom, "bottom row must be part of the wave");
  }

  #[test]
  fn colours_stay_red() {
    for (_, color) in frame(0.4) {
      assert!(color.r >= color.g && color.r >= color.b);
    }
  }
}
