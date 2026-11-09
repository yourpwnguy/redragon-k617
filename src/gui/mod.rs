//! Skinned GUI for the K617, modelled on the vendor MFC application.
//!
//! The window is a fixed 1000x640 canvas using the original `main_nr.png`
//! background. Every control is painted at the coordinates recovered from
//! `Cfg.ini` and from the running 1.6.6 app, so the layout matches the Windows
//! software rather than a generic desktop form.

pub mod app;
pub mod assets;
pub mod i18n;
pub mod key_rects;
pub mod keyboard;
pub mod macro_editor;
pub mod widgets;

pub use app::App;
pub use i18n::Lang;
pub use macro_editor::MacroEditor;

use crate::effects::Effect;
use crate::protocol::Rgb;

/// A user intent produced by the painted widgets. The app applies these after
/// the frame is drawn so borrows stay short and predictable.
#[derive(Clone, Copy, Debug)]
pub enum GameAction {
  None,
  ToggleLight,
  ToggleMode,
  SelectEffect(Effect),
  ToggleLang,
  SetLang(Lang),
  ToggleRgb,
  ToggleColorPicker,
  SetColor(Rgb),
  SetSpeed(u8),
  SetBrightness(u8),
  SelectKey(usize, bool),
  SelectProfile(usize),
  OpenMacro,
  Restore,
  Export,
  Import,
  Apply,
  Ok,
  Cancel,
  Minimize,
  Close,
  OpenWeb,
  ConfirmRestore(bool),
}

pub fn run() -> eframe::Result<()> {
  let icon = image::load_from_memory(include_bytes!("../../assets/gui/appico.png"))
    .ok()
    .map(|image| {
      let image = image.to_rgba8();
      eframe::egui::IconData {
        rgba: image.as_raw().clone(),
        width: image.width(),
        height: image.height(),
      }
    });
  let mut viewport = eframe::egui::ViewportBuilder::default()
    .with_inner_size(app::WINDOW_SIZE)
    .with_decorations(false)
    .with_transparent(true)
    .with_resizable(false)
    .with_title("Redragon K617-RGB");
  if let Some(icon) = icon {
    viewport = viewport.with_icon(icon);
  }
  let options = eframe::NativeOptions {
    viewport,
    ..Default::default()
  };
  eframe::run_native(
    "Redragon K617-RGB",
    options,
    Box::new(|cc| Ok(Box::new(App::new(cc)))),
  )
}
