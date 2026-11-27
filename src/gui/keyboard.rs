//! Keyboard preview and per-key hit testing.
//!
//! The key rectangles come from the vendor `Cfg.ini` `[KEY]` section, so the
//! preview is not an approximation: Esc is at (13,14)-(37,42) inside the
//! 560x212 keyboard image, exactly like the Windows app. Clicking a key
//! selects it for the "current function" readout. In Self-define mode the
//! old report-8 path used to paint the key, but that frame can reset some
//! firmware revisions, so the GUI now shows a "preview-only" warning and
//! leaves the device alone.

use eframe::egui::{self, Color32, Id, Rect, Sense, Stroke, Ui};

use crate::keymap;
use crate::protocol::Rgb;

use super::assets::{paint_texture, Assets};
use super::key_rects::KEY_RECTS;
use super::widgets;

pub const KEYBOARD_ORIGIN: egui::Pos2 = egui::pos2(350.0, 80.0);

#[derive(Clone, Copy, Debug)]
pub struct KeyClick {
  pub index: usize,
  pub right: bool,
}

/// Resolve the vendor short name to our canonical key name.
fn canonical(name: &str) -> &str {
  match name {
    "Esc" => "Escape",
    "App" => "Menu",
    "LWin" => "LSuper",
    ";" => "Semicolon",
    "'" => "Quote",
    "," => "Comma",
    "." => "Period",
    "/" => "Slash",
    "\\" => "Backslash",
    "[" => "LBracket",
    "]" => "RBracket",
    "-" => "Minus",
    "=" => "Equal",
    other => match other {
      "1" => "1",
      "2" => "2",
      "3" => "3",
      "4" => "4",
      "5" => "5",
      "6" => "6",
      "7" => "7",
      "8" => "8",
      "9" => "9",
      "0" => "0",
      "Q" => "Q",
      "W" => "W",
      "E" => "E",
      "R" => "R",
      "T" => "T",
      "Y" => "Y",
      "U" => "U",
      "I" => "I",
      "O" => "O",
      "P" => "P",
      "A" => "A",
      "S" => "S",
      "D" => "D",
      "F" => "F",
      "G" => "G",
      "H" => "H",
      "J" => "J",
      "K" => "K",
      "L" => "L",
      "Z" => "Z",
      "X" => "X",
      "C" => "C",
      "V" => "V",
      "B" => "B",
      "N" => "N",
      "M" => "M",
      "Tab" => "Tab",
      "CapsLock" => "CapsLock",
      "Backspace" => "Backspace",
      "Enter" => "Enter",
      "LShift" => "LShift",
      "RShift" => "RShift",
      "LCtrl" => "LCtrl",
      "RCtrl" => "RCtrl",
      "LAlt" => "LAlt",
      "RAlt" => "RAlt",
      "Space" => "Space",
      "Fn" => "Fn",
      _ => other,
    },
  }
}

/// Draw the keyboard image and return the key the user clicked.
///
/// `tint` is `Some` only for Self-define with RGB enabled. This is a local
/// preview: the GUI does not send per-key colours to the device because the
/// report-8 path can reset some K617 firmware revisions. The vendor app uses
/// report-6 colour groups instead.
pub fn draw(
  ui: &Ui,
  assets: &Assets,
  colors: Option<&[Rgb; keymap::KEY_COUNT]>,
  selected: Option<usize>,
) -> Option<KeyClick> {
  let origin = KEYBOARD_ORIGIN;
  let image_rect = Rect::from_min_size(origin, egui::vec2(560.0, 212.0));
  paint_texture(ui.painter(), &assets.keyboard, image_rect, Color32::WHITE);

  let mut clicked = None;
  for (index, (name, x1, y1, x2, y2)) in KEY_RECTS.iter().enumerate() {
    let rect = Rect::from_min_max(
      origin + egui::vec2(*x1 as f32, *y1 as f32),
      origin + egui::vec2(*x2 as f32, *y2 as f32),
    );
    if let Some(colors) = colors {
      let color = colors[index];
      let inset = rect.shrink(3.0);
      ui.painter().rect_filled(
        inset,
        3.0,
        Color32::from_rgba_unmultiplied(color.r, color.g, color.b, 150),
      );
      // Redraw the original keycap over the tint so the legend and the bevel
      // stay readable, which is how the vendor preview looks.
      let uv = Rect::from_min_max(
        egui::pos2(*x1 as f32 / 560.0, *y1 as f32 / 212.0),
        egui::pos2(*x2 as f32 / 560.0, *y2 as f32 / 212.0),
      );
      ui.painter().image(
        assets.keyboard.id(),
        rect,
        uv,
        Color32::from_white_alpha(80),
      );
    }

    let response = ui.interact(rect, Id::unique(("key", index)), Sense::click());
    if response.hovered() {
      ui.painter().rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, Color32::from_rgb(255, 90, 90)),
        egui::StrokeKind::Inside,
      );
    }
    if selected == Some(index) {
      ui.painter().rect_stroke(
        rect.shrink(1.0),
        2.0,
        Stroke::new(1.5, Color32::WHITE),
        egui::StrokeKind::Inside,
      );
    }
    if response.clicked() {
      clicked = Some(KeyClick {
        index,
        right: false,
      });
    }
    if response.secondary_clicked() {
      clicked = Some(KeyClick { index, right: true });
    }
    let key_name = canonical(name);
    response.on_hover_text(key_name);
  }
  clicked
}

/// Convert a led index to the canonical key name.
pub fn name_of(index: usize) -> &'static str {
  keymap::KEYS.get(index).map(|key| key.name).unwrap_or("?")
}

/// Render the hint under the keyboard, matching the vendor line.
pub fn draw_hint(ui: &Ui, text: &str) {
  widgets::text(ui, egui::pos2(350.0, 301.0), text, 12.0, widgets::TEXT_DIM);
}
