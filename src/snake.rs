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

use crate::keymap::{KeyDef, KEYS};
use crate::protocol::Rgb;

/// Vertical centre of the wave in matrix rows (0 is the top row).
const CENTRE: f32 = 2.55;
/// How far the centreline swings above and below `CENTRE`.
const AMPLITUDE: f32 = 1.35;
/// Number of full S curves across the used keyboard width.
const PERIODS: f32 = 1.5;
/// Base half-thickness in matrix rows.
const THICKNESS: f32 = 1.08;

/// One animation frame. Returns one colour per K617 key. Unlit keys are black.
pub fn frame(phase: f32) -> Vec<(KeyDef, Rgb)> {
  KEYS
    .iter()
    .map(|key| {
      let slot = key.sinodragon_slot as f32;
      let col = (slot / 6.0).floor();
      let row = slot % 6.0;
      // The K617 uses columns 0..13; 14 and 15 are empty slots.
      let x = (col / 13.0).clamp(0.0, 1.0);
      let centre = CENTRE + AMPLITUDE * (TAU * PERIODS * x - phase).sin();
      let distance = (row - centre).abs();

      // Two low-frequency terms give a slightly ragged edge. The amplitude
      // is deliberately small: enough to read as distressed, not noisy.
      let thickness = THICKNESS
        + 0.12 * (col * 1.31 + phase * 1.7).sin()
        + 0.06 * (col * 0.57 - row * 0.82 + phase * 2.1).sin();
      if distance >= thickness {
        return (*key, Rgb::OFF);
      }

      let intensity = (1.0 - distance / thickness).powf(0.72);
      // Keep the core bright red and fade the border through dark red. The
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
  fn colours_stay_red() {
    for (_, color) in frame(0.4) {
      assert!(color.r >= color.g && color.r >= color.b);
    }
  }
}
