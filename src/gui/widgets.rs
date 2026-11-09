//! Small painted widgets that keep the vendor skin geometry.
//!
//! egui's stock widgets have their own borders and paddings, which would
//! immediately make the window look different from the MFC app. These helpers
//! draw the original PNG skins into an exact rectangle and do their own hit
//! testing.

use eframe::egui::{self, Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui};

use super::assets::{paint_texture, Assets};

pub const TEXT: Color32 = Color32::from_rgb(220, 220, 220);
pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 150, 150);
pub const TEXT_BUTTON: Color32 = Color32::from_rgb(200, 200, 200);
pub const TEXT_BUTTON_HOVER: Color32 = Color32::from_rgb(255, 255, 255);
pub const ACCENT: Color32 = Color32::from_rgb(220, 20, 20);
pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(255, 40, 40);
pub const LIST_BG: Color32 = Color32::from_rgb(255, 255, 255);
pub const LIST_TEXT: Color32 = Color32::from_rgb(20, 20, 20);
pub const PANEL: Color32 = Color32::from_rgb(28, 28, 30);

pub fn font(size: f32) -> FontId {
  FontId::proportional(size)
}

pub fn text(ui: &Ui, pos: egui::Pos2, value: &str, size: f32, color: Color32) {
  ui.painter()
    .text(pos, Align2::LEFT_TOP, value, font(size), color);
}

pub fn text_centered(ui: &Ui, rect: Rect, value: &str, size: f32, color: Color32) {
  ui.painter().text(
    rect.center(),
    Align2::CENTER_CENTER,
    value,
    font(size),
    color,
  );
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
  Normal,
  Wide,
  Selected,
}

/// A skinned push button. The rectangle is in window coordinates.
pub fn button(
  ui: &Ui,
  assets: &Assets,
  rect: Rect,
  label: &str,
  kind: ButtonKind,
) -> egui::Response {
  let id = Id::unique(("button", rect.min.x.to_bits(), rect.min.y.to_bits(), label));
  let response = ui.interact(rect, id, Sense::click());
  let hovered = response.hovered();
  let pressed = response.is_pointer_button_down_on();

  match kind {
    ButtonKind::Selected => {
      paint_texture(
        ui.painter(),
        &assets.button_dn,
        rect,
        Color32::from_rgb(255, 90, 90),
      );
    }
    ButtonKind::Wide => {
      let texture = if pressed {
        &assets.button_dn
      } else {
        &assets.button_nr
      };
      paint_texture(ui.painter(), texture, rect, Color32::WHITE);
    }
    ButtonKind::Normal => {
      let texture = if hovered {
        &assets.btn_ov
      } else {
        &assets.btn_nr
      };
      paint_texture(ui.painter(), texture, rect, Color32::WHITE);
    }
  }

  let color = if hovered || pressed {
    TEXT_BUTTON_HOVER
  } else {
    TEXT_BUTTON
  };
  text_centered(ui, rect, label, 12.0, color);
  response
}

pub fn icon_button(
  ui: &Ui,
  _assets: &Assets,
  rect: Rect,
  normal: &egui::TextureHandle,
  hover: &egui::TextureHandle,
) -> egui::Response {
  let id = Id::unique(("icon", rect.min.x.to_bits(), rect.min.y.to_bits()));
  let response = ui.interact(rect, id, Sense::click());
  let texture = if response.hovered() { hover } else { normal };
  paint_texture(ui.painter(), texture, rect, Color32::WHITE);
  response
}

pub fn checkbox(
  ui: &Ui,
  assets: &Assets,
  rect: Rect,
  checked: bool,
  label: &str,
) -> egui::Response {
  let id = Id::unique(("check", rect.min.x.to_bits(), rect.min.y.to_bits(), label));
  let response = ui.interact(rect, id, Sense::click());
  let texture = if checked {
    &assets.check_dn
  } else if response.hovered() {
    &assets.check_ov
  } else {
    &assets.check_nr
  };
  paint_texture(ui.painter(), texture, rect, Color32::WHITE);
  let text_pos = egui::pos2(rect.max.x + 6.0, rect.center().y - 7.0);
  text(ui, text_pos, label, 12.0, TEXT);
  response
}

pub fn checkbox_size(
  ui: &Ui,
  assets: &Assets,
  rect: Rect,
  checked: bool,
  label: &str,
  font_size: f32,
) -> egui::Response {
  let id = Id::unique(("check", rect.min.x.to_bits(), rect.min.y.to_bits(), label));
  let response = ui.interact(rect, id, Sense::click());
  let texture = if checked {
    &assets.check_dn
  } else if response.hovered() {
    &assets.check_ov
  } else {
    &assets.check_nr
  };
  paint_texture(ui.painter(), texture, rect, Color32::WHITE);
  let text_pos = egui::pos2(rect.max.x + 6.0, rect.center().y - font_size * 0.55);
  text(ui, text_pos, label, font_size, TEXT);
  response
}

pub fn radio(ui: &Ui, assets: &Assets, rect: Rect, selected: bool, label: &str) -> egui::Response {
  let id = Id::unique(("radio", rect.min.x.to_bits(), rect.min.y.to_bits(), label));
  let response = ui.interact(rect, id, Sense::click());
  let texture = if selected {
    &assets.check_dn
  } else if response.hovered() {
    &assets.check_ov
  } else {
    &assets.check_nr
  };
  paint_texture(ui.painter(), texture, rect, Color32::WHITE);
  text(
    ui,
    egui::pos2(rect.max.x + 6.0, rect.center().y - 7.0),
    label,
    12.0,
    TEXT,
  );
  response
}

/// Horizontal skinned slider. Returns `Some(new_value)` when the drag changed
/// it. `min`/`max` are inclusive and the knob snaps to integers.
pub fn slider(ui: &Ui, assets: &Assets, track: Rect, value: u8, min: u8, max: u8) -> Option<u8> {
  let id = Id::unique(("slider", track.min.x.to_bits(), track.min.y.to_bits()));
  let response = ui.interact(
    Rect::from_min_max(
      egui::pos2(track.min.x, track.min.y - 8.0),
      egui::pos2(track.max.x, track.max.y + 8.0),
    ),
    id,
    Sense::click_and_drag(),
  );

  ui.painter().line_segment(
    [track.left_center(), track.right_center()],
    Stroke::new(3.0, ACCENT),
  );

  let span = (max - min).max(1) as f32;
  let fraction = (value.saturating_sub(min)) as f32 / span;
  let knob_x = track.min.x + fraction * (track.width() - 2.0) + 1.0;
  let knob_rect =
    Rect::from_center_size(egui::pos2(knob_x, track.center().y), egui::vec2(16.0, 15.0));

  if response.dragged() || response.clicked() {
    if let Some(pointer) = response.interact_pointer_pos() {
      let relative = ((pointer.x - track.min.x) / track.width()).clamp(0.0, 1.0);
      let new_value = min + (relative * span).round() as u8;
      return Some(new_value.clamp(min, max));
    }
  }

  paint_texture(ui.painter(), &assets.slider, knob_rect, Color32::WHITE);
  None
}

pub fn combo(ui: &Ui, assets: &Assets, rect: Rect, value: &str, open: bool) -> egui::Response {
  let id = Id::unique(("combo", rect.min.x.to_bits(), rect.min.y.to_bits(), value));
  let response = ui.interact(rect, id, Sense::click());
  let texture = if open || response.hovered() {
    &assets.combo_ov
  } else {
    &assets.combo_nr
  };
  paint_texture(ui.painter(), texture, rect, Color32::WHITE);
  text(
    ui,
    egui::pos2(rect.min.x + 6.0, rect.center().y - 7.0),
    value,
    12.0,
    TEXT,
  );
  ui.painter().text(
    egui::pos2(rect.max.x - 10.0, rect.center().y),
    Align2::CENTER_CENTER,
    "▾",
    font(10.0),
    TEXT,
  );
  response
}

/// Dark popup list used by the effect/language dropdowns.
pub fn popup_list(ui: &Ui, rect: Rect, rows: &[String], selected: Option<usize>) -> Option<usize> {
  ui.painter()
    .rect_filled(rect, 2.0, Color32::from_rgb(36, 36, 38));
  ui.painter().rect_stroke(
    rect,
    2.0,
    Stroke::new(1.0, Color32::from_rgb(80, 80, 82)),
    egui::StrokeKind::Inside,
  );
  let row_height = rect.height() / rows.len() as f32;
  let mut clicked = None;
  for (index, row) in rows.iter().enumerate() {
    let row_rect = Rect::from_min_size(
      egui::pos2(rect.min.x + 2.0, rect.min.y + index as f32 * row_height),
      egui::vec2(rect.width() - 4.0, row_height),
    );
    let response = ui.interact(
      row_rect,
      Id::unique(("popup_row", rect.min.x.to_bits(), index)),
      Sense::click(),
    );
    if response.hovered() {
      ui.painter().rect_filled(row_rect, 1.0, ACCENT);
    }
    let color = if selected == Some(index) {
      Color32::from_rgb(255, 180, 180)
    } else {
      TEXT
    };
    text(
      ui,
      egui::pos2(row_rect.min.x + 5.0, row_rect.center().y - 7.0),
      row,
      12.0,
      color,
    );
    if response.clicked() {
      clicked = Some(index);
    }
  }
  clicked
}
