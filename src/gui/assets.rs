//! Embedded skin textures.
//!
//! Everything is loaded once at startup from the vendor skin pack. The
//! textures are scaled to the original control rectangles, so the GUI keeps
//! the same pixel geometry as the Windows app.

use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};

pub struct Assets {
  pub main_bg: TextureHandle,
  pub keyboard: TextureHandle,
  pub macro_bg: TextureHandle,
  pub button_nr: TextureHandle,
  pub button_dn: TextureHandle,
  pub btn_nr: TextureHandle,
  pub btn_ov: TextureHandle,
  pub combo_nr: TextureHandle,
  pub combo_ov: TextureHandle,
  pub slider: TextureHandle,
  pub check_nr: TextureHandle,
  pub check_ov: TextureHandle,
  pub check_dn: TextureHandle,
  pub min_nr: TextureHandle,
  pub min_ov: TextureHandle,
  pub exit_nr: TextureHandle,
  pub exit_ov: TextureHandle,
  pub recbtn_nr: TextureHandle,
  pub recbtn_dn: TextureHandle,
  pub rcolor: TextureHandle,
}

impl Assets {
  pub fn load(ctx: &egui::Context) -> Self {
    Self {
      main_bg: tex(
        ctx,
        "main_bg",
        include_bytes!("../../assets/gui/main_nr.png"),
      ),
      keyboard: tex(
        ctx,
        "keyboard",
        include_bytes!("../../assets/gui/keyimg.png"),
      ),
      macro_bg: tex(
        ctx,
        "macro_bg",
        include_bytes!("../../assets/gui/macro_nr.png"),
      ),
      button_nr: tex(
        ctx,
        "button_nr",
        include_bytes!("../../assets/gui/button_nr.png"),
      ),
      button_dn: tex(
        ctx,
        "button_dn",
        include_bytes!("../../assets/gui/button_dn.png"),
      ),
      btn_nr: tex(ctx, "btn_nr", include_bytes!("../../assets/gui/btn_nr.png")),
      btn_ov: tex(ctx, "btn_ov", include_bytes!("../../assets/gui/btn_ov.png")),
      combo_nr: tex(
        ctx,
        "combo_nr",
        include_bytes!("../../assets/gui/combo_nr.png"),
      ),
      combo_ov: tex(
        ctx,
        "combo_ov",
        include_bytes!("../../assets/gui/combo_ov.png"),
      ),
      slider: tex(ctx, "slider", include_bytes!("../../assets/gui/slider.png")),
      check_nr: tex(
        ctx,
        "check_nr",
        include_bytes!("../../assets/gui/check_nr.png"),
      ),
      check_ov: tex(
        ctx,
        "check_ov",
        include_bytes!("../../assets/gui/check_ov.png"),
      ),
      check_dn: tex(
        ctx,
        "check_dn",
        include_bytes!("../../assets/gui/check_dn.png"),
      ),
      min_nr: tex(ctx, "min_nr", include_bytes!("../../assets/gui/min_nr.png")),
      min_ov: tex(ctx, "min_ov", include_bytes!("../../assets/gui/min_ov.png")),
      exit_nr: tex(
        ctx,
        "exit_nr",
        include_bytes!("../../assets/gui/exit_nr.png"),
      ),
      exit_ov: tex(
        ctx,
        "exit_ov",
        include_bytes!("../../assets/gui/exit_ov.png"),
      ),
      recbtn_nr: tex(
        ctx,
        "recbtn_nr",
        include_bytes!("../../assets/gui/recbtn_nr.png"),
      ),
      recbtn_dn: tex(
        ctx,
        "recbtn_dn",
        include_bytes!("../../assets/gui/recbtn_dn.png"),
      ),
      rcolor: tex(ctx, "rcolor", include_bytes!("../../assets/gui/rcolor.png")),
    }
  }
}

fn tex(ctx: &egui::Context, name: &str, bytes: &[u8]) -> TextureHandle {
  let image = image::load_from_memory(bytes)
    .unwrap_or_else(|error| panic!("embedded skin {name}: {error}"))
    .to_rgba8();
  let size = [image.width() as usize, image.height() as usize];
  let pixels = ColorImage::from_rgba_unmultiplied(size, image.as_raw());
  ctx.load_texture(name, pixels, TextureOptions::LINEAR)
}

/// UV rectangle for a whole texture.
pub const FULL_UV: egui::Rect =
  egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

pub fn paint_texture(
  painter: &egui::Painter,
  texture: &TextureHandle,
  rect: egui::Rect,
  tint: Color32,
) {
  painter.image(texture.id(), rect, FULL_UV, tint);
}
