//! Main window: title bar, profile bar, keyboard preview, Light panel and the
//! vendor footer buttons.
//!
//! Coordinates are taken from the vendor `Cfg.ini` (`rcKB`, `rcLED`,
//! `ptProfile`, `ptMacro`, `ptApplyBtn`, `BarW`, `BarH`) and from the running
//! 1.6.6 app under Wine. The window is a fixed 1000x640 pixel canvas, exactly
//! like `main_nr.png`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui, ViewportCommand};

use crate::effects::Effect;
use crate::hid::Keyboard;
use crate::profile::{self, Profile};
use crate::protocol::{self, EffectParams, Rgb};

use super::assets::{paint_texture, Assets};
use super::i18n::{I18n, Lang};
use super::keyboard;
use super::widgets::{self, ButtonKind};
use super::GameAction;
use super::MacroEditor;

pub const WINDOW_SIZE: [f32; 2] = [1000.0, 640.0];
const PRESET_COLORS: [Rgb; 9] = [
  Rgb { r: 255, g: 0, b: 0 },
  Rgb { r: 0, g: 255, b: 0 },
  Rgb { r: 0, g: 0, b: 255 },
  Rgb {
    r: 255,
    g: 0,
    b: 255,
  },
  Rgb { r: 0, g: 128, b: 0 },
  Rgb {
    r: 0,
    g: 255,
    b: 255,
  },
  Rgb {
    r: 255,
    g: 255,
    b: 0,
  },
  Rgb {
    r: 255,
    g: 128,
    b: 0,
  },
  Rgb {
    r: 255,
    g: 255,
    b: 255,
  },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileMode {
  Export,
  Import,
}

struct FileDialog {
  mode: FileMode,
  path: String,
  error: Option<String>,
}

pub struct App {
  pub assets: Assets,
  pub i18n_en: I18n,
  pub i18n_sc: I18n,
  pub lang: Lang,

  pub profiles: [Profile; 3],
  pub active_profile: usize,
  pub draft: Profile,
  pub light_open: bool,
  pub rgb_enabled: bool,
  pub selected_key: Option<usize>,

  pub device: Option<Keyboard>,
  pub device_error: Option<String>,
  pub status: String,
  pub status_until: Option<Instant>,

  pub dirty: bool,
  pub last_change: Instant,

  pub mode_open: bool,
  pub lang_open: bool,
  pub color_open: bool,
  pub rcolor_pixels: Vec<u8>,
  pub rcolor_size: [usize; 2],

  file_dialog: Option<FileDialog>,
  pub confirm_restore: bool,

  pub macro_editor: MacroEditor,
}

impl App {
  pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    install_fonts(&cc.egui_ctx);
    let assets = Assets::load(&cc.egui_ctx);
    let i18n_en = I18n::load(Lang::English);
    let i18n_sc = I18n::load(Lang::Chinese);

    let mut profiles = load_profiles();
    if let Ok(Some(current)) = profile::load_current() {
      profiles[0] = current;
    }
    let draft = profiles[0].clone();

    let rcolor = image::load_from_memory(include_bytes!("../../assets/gui/rcolor.png"))
      .expect("rcolor")
      .to_rgba8();
    let rcolor_size = [rcolor.width() as usize, rcolor.height() as usize];
    let rcolor_pixels = rcolor.as_raw().clone();

    let mut app = Self {
      assets,
      i18n_en,
      i18n_sc,
      lang: Lang::English,
      profiles,
      active_profile: 0,
      draft,
      light_open: false,
      rgb_enabled: true,
      selected_key: None,
      device: None,
      device_error: None,
      status: String::new(),
      status_until: None,
      dirty: false,
      last_change: Instant::now(),
      mode_open: false,
      lang_open: false,
      color_open: false,
      rcolor_pixels,
      rcolor_size,
      file_dialog: None,
      confirm_restore: false,
      macro_editor: MacroEditor::load(),
    };
    app.connect();
    app
  }

  pub fn t<'a>(&'a self, key: &str) -> &'a str {
    let primary = match self.lang {
      Lang::English => &self.i18n_en,
      Lang::Chinese => &self.i18n_sc,
    };
    if primary.has(key) {
      primary.t(key)
    } else {
      self.i18n_en.t(key)
    }
  }

  fn mode_name(&self, effect: Effect) -> String {
    let key = format!("tc_led_mode{}", effect.number());
    self.t(&key).to_string()
  }

  fn connect(&mut self) {
    match Keyboard::open() {
      Ok(keyboard) => {
        if let Ok(state) = keyboard.read_state() {
          if state.len() > protocol::STATE_MODE {
            let mode = state[protocol::STATE_MODE];
            if let Some(effect) = Effect::ALL.into_iter().find(|e| e.number() == mode) {
              self.draft.effect = effect.name().to_string();
            }
            let packed = state[protocol::STATE_SPEED_BRIGHT[0]];
            let speed = (packed >> 4) & 0xf;
            let brightness = packed & 0xf;
            if (1..=4).contains(&speed) {
              self.draft.speed = speed;
            }
            if brightness <= 4 {
              self.draft.brightness = brightness;
            }
          }
        }
        self.device = Some(keyboard);
        self.device_error = None;
      }
      Err(error) => {
        self.device = None;
        self.device_error = Some(format!("{error}"));
      }
    }
  }

  fn mark_dirty(&mut self) {
    self.dirty = true;
    self.last_change = Instant::now();
  }

  fn apply_draft(&mut self) {
    if self.device.is_none() {
      self.connect();
    }
    let Some(keyboard) = self.device.as_ref() else {
      return;
    };
    let effect = self.draft.effect().unwrap_or(Effect::FixedOn);
    let color = self.draft.color().unwrap_or(Rgb::new(255, 0, 0));
    let params = EffectParams {
      effect,
      color,
      speed: self.draft.speed,
      brightness: self.draft.brightness,
    };
    // We deliberately do not send the non-vendor report-8 per-key frame
    // here. The official app uses report 6 LED-matrix blocks, and report 8
    // has been observed to reset some K617 firmware revisions when the base
    // colour changes. The GUI preview still shows the per-key map, but the
    // device only receives the safe five-frame firmware burst.
    let result = (|| -> anyhow::Result<()> {
      keyboard.apply_effect(params)?;
      Ok(())
    })();
    match result {
      Ok(()) => {
        self.device_error = None;
        self.status.clear();
        self.status_until = None;
      }
      Err(error) => {
        self.device_error = Some(format!("{error:#}"));
        self.device = None;
      }
    }
  }

  fn set_status(&mut self, message: String, seconds: u64) {
    if message.is_empty() {
      return;
    }
    self.status = message;
    self.status_until = Some(Instant::now() + Duration::from_secs(seconds));
  }

  fn save_active(&mut self) {
    self.profiles[self.active_profile] = self.draft.clone();
    let name = format!("Profile {}", self.active_profile + 1);
    let _ = profile::save_named(&name, &self.draft);
    let _ = profile::save_current(&self.draft);
  }

  fn switch_profile(&mut self, index: usize) {
    if index == self.active_profile {
      return;
    }
    self.profiles[self.active_profile] = self.draft.clone();
    let _ = profile::save_named(&format!("Profile {}", self.active_profile + 1), &self.draft);
    self.active_profile = index;
    self.draft = self.profiles[index].clone();
    self.selected_key = None;
    self.mark_dirty();
  }

  fn restore_defaults(&mut self) {
    self.draft = Profile::new(Effect::FixedOn, Rgb::new(255, 0, 0), 3, 3);
    self.selected_key = None;
    self.mark_dirty();
  }

  fn profile_colors(&self) -> [Rgb; crate::keymap::KEY_COUNT] {
    let base = self.draft.color().unwrap_or(Rgb::new(255, 0, 0));
    let mut colors = [base; crate::keymap::KEY_COUNT];
    for (name, value) in &self.draft.keys {
      if let Some(key) = crate::keymap::find(name) {
        if let Some(color) = Rgb::parse(value) {
          colors[key.led_index] = color;
        }
      }
    }
    colors
  }

  fn handle_action(&mut self, action: GameAction, ctx: &egui::Context) {
    match action {
      GameAction::None => {}
      GameAction::ToggleLight => {
        self.light_open = !self.light_open;
      }
      GameAction::ToggleMode => {
        self.mode_open = !self.mode_open;
        self.lang_open = false;
        self.color_open = false;
      }
      GameAction::SelectEffect(effect) => {
        self.draft.effect = effect.name().to_string();
        self.mode_open = false;
        self.mark_dirty();
      }
      GameAction::ToggleLang => {
        self.lang_open = !self.lang_open;
        self.mode_open = false;
        self.color_open = false;
      }
      GameAction::SetLang(lang) => {
        self.lang = lang;
        self.lang_open = false;
      }
      GameAction::ToggleRgb => {
        self.rgb_enabled = !self.rgb_enabled;
        self.mark_dirty();
      }
      GameAction::ToggleColorPicker => {
        self.color_open = !self.color_open;
        self.mode_open = false;
        self.lang_open = false;
      }
      GameAction::SetColor(color) => {
        self.draft.color = color.to_hex();
        self.mark_dirty();
      }
      GameAction::SetSpeed(value) => {
        self.draft.speed = value.clamp(1, 4);
        self.mark_dirty();
      }
      GameAction::SetBrightness(value) => {
        self.draft.brightness = value.min(4);
        self.mark_dirty();
      }
      GameAction::SelectKey(index, right) => {
        self.selected_key = Some(index);
        if right {
          let name = keyboard::name_of(index);
          if self.draft.keys.remove(name).is_some() {
            self.mark_dirty();
          }
        } else if self.draft.effect().ok() == Some(Effect::SelfDefine) {
          let message = self.t("tc_perkey_preview_only").to_string();
          self.set_status(message, 4);
        }
      }
      GameAction::SelectProfile(index) => self.switch_profile(index),
      GameAction::OpenMacro => self.macro_editor.open = true,
      GameAction::Restore => self.confirm_restore = true,
      GameAction::Export => {
        let path = default_export_path();
        self.file_dialog = Some(FileDialog {
          mode: FileMode::Export,
          path,
          error: None,
        });
      }
      GameAction::Import => {
        let path = default_export_path();
        self.file_dialog = Some(FileDialog {
          mode: FileMode::Import,
          path,
          error: None,
        });
      }
      GameAction::Apply => {
        self.save_active();
        self.apply_draft();
      }
      GameAction::Ok => {
        self.save_active();
        self.apply_draft();
        ctx.send_viewport_cmd(ViewportCommand::Close);
      }
      GameAction::Cancel => {
        self.draft = self.profiles[self.active_profile].clone();
        self.apply_draft();
        ctx.send_viewport_cmd(ViewportCommand::Close);
      }
      GameAction::Minimize => {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
      }
      GameAction::Close => {
        self.save_active();
        self.apply_draft();
        ctx.send_viewport_cmd(ViewportCommand::Close);
      }
      GameAction::OpenWeb => {
        let _ = std::process::Command::new("xdg-open")
          .arg("https://www.redragonzone.com")
          .spawn();
      }
      GameAction::ConfirmRestore(confirmed) => {
        self.confirm_restore = false;
        if confirmed {
          self.restore_defaults();
        }
      }
    }
  }

  fn draw_title_bar(&mut self, ui: &mut Ui) -> Vec<GameAction> {
    let mut actions = Vec::new();
    // Window drag area, excluding the three title-bar controls.
    let drag_rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(795.0, 40.0));
    let drag = ui.interact(drag_rect, Id::unique("title_drag"), Sense::click_and_drag());
    if drag.dragged() {
      ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let lang_rect = Rect::from_min_size(egui::pos2(800.0, 27.0), egui::vec2(94.0, 20.0));
    if widgets::combo(
      ui,
      &self.assets,
      lang_rect,
      self.lang.label(),
      self.lang_open,
    )
    .clicked()
    {
      actions.push(GameAction::ToggleLang);
    }

    let min_rect = Rect::from_min_size(egui::pos2(900.0, 27.0), egui::vec2(19.0, 18.0));
    if widgets::icon_button(
      ui,
      &self.assets,
      min_rect,
      &self.assets.min_nr,
      &self.assets.min_ov,
    )
    .clicked()
    {
      actions.push(GameAction::Minimize);
    }

    let close_rect = Rect::from_min_size(egui::pos2(935.0, 27.0), egui::vec2(19.0, 18.0));
    if widgets::icon_button(
      ui,
      &self.assets,
      close_rect,
      &self.assets.exit_nr,
      &self.assets.exit_ov,
    )
    .clicked()
    {
      actions.push(GameAction::Close);
    }
    actions
  }

  fn draw_profile_bar(&mut self, ui: &mut Ui) -> Vec<GameAction> {
    let mut actions = Vec::new();
    for index in 0..3 {
      let rect = Rect::from_min_size(
        egui::pos2(70.0, 169.0 + index as f32 * 38.0),
        egui::vec2(151.0, 28.0),
      );
      let label = format!("{} {}", self.t("tc_proname"), index + 1);
      let kind = if index == self.active_profile {
        ButtonKind::Selected
      } else {
        ButtonKind::Wide
      };
      if widgets::button(ui, &self.assets, rect, &label, kind).clicked() {
        actions.push(GameAction::SelectProfile(index));
      }
    }
    let macro_rect = Rect::from_min_size(egui::pos2(70.0, 317.0), egui::vec2(151.0, 28.0));
    if widgets::button(
      ui,
      &self.assets,
      macro_rect,
      self.t("tc_page2"),
      ButtonKind::Wide,
    )
    .clicked()
    {
      actions.push(GameAction::OpenMacro);
    }
    actions
  }

  fn draw_light_panel(&mut self, ui: &mut Ui) -> Vec<GameAction> {
    let mut actions = Vec::new();
    let check_rect = Rect::from_min_size(egui::pos2(322.0, 371.0), egui::vec2(13.0, 12.0));
    if widgets::checkbox(
      ui,
      &self.assets,
      check_rect,
      self.light_open,
      self.t("tc_msg1"),
    )
    .clicked()
    {
      actions.push(GameAction::ToggleLight);
    }
    if !self.light_open {
      return actions;
    }

    let effect = self.draft.effect().unwrap_or(Effect::FixedOn);
    let mode_rect = Rect::from_min_size(egui::pos2(404.0, 375.0), egui::vec2(169.0, 16.0));
    let mode_text = self.mode_name(effect);
    if widgets::combo(ui, &self.assets, mode_rect, &mode_text, self.mode_open).clicked() {
      actions.push(GameAction::ToggleMode);
    }

    widgets::text(
      ui,
      egui::pos2(674.0, 376.0),
      self.t("tc_adv_str12"),
      12.0,
      widgets::TEXT,
    );
    let swatch_rect = Rect::from_min_size(egui::pos2(712.0, 375.0), egui::vec2(58.0, 16.0));
    let color = self.draft.color().unwrap_or(Rgb::new(255, 0, 0));
    ui.painter()
      .rect_filled(swatch_rect, 0.0, Color32::from_rgb(20, 20, 20));
    ui.painter().rect_stroke(
      swatch_rect,
      0.0,
      Stroke::new(1.0, Color32::from_rgb(200, 200, 200)),
      egui::StrokeKind::Inside,
    );
    let fill_rect = Rect::from_min_size(
      egui::pos2(swatch_rect.min.x + 2.0, swatch_rect.min.y + 2.0),
      egui::vec2(swatch_rect.width() - 18.0, swatch_rect.height() - 4.0),
    );
    ui.painter()
      .rect_filled(fill_rect, 0.0, Color32::from_rgb(color.r, color.g, color.b));
    let arrow_rect = Rect::from_min_size(
      egui::pos2(swatch_rect.max.x - 14.0, swatch_rect.min.y + 1.0),
      egui::vec2(13.0, swatch_rect.height() - 2.0),
    );
    ui.painter()
      .rect_filled(arrow_rect, 0.0, Color32::from_rgb(130, 130, 130));
    ui.painter().text(
      arrow_rect.center(),
      Align2::CENTER_CENTER,
      "▾",
      FontId::proportional(10.0),
      Color32::from_rgb(30, 30, 30),
    );
    let swatch = ui.interact(swatch_rect, Id::unique("color_swatch"), Sense::click());
    if swatch.clicked() {
      actions.push(GameAction::ToggleColorPicker);
    }

    let rgb_check = Rect::from_min_size(egui::pos2(795.0, 371.0), egui::vec2(13.0, 12.0));
    if widgets::checkbox(
      ui,
      &self.assets,
      rgb_check,
      self.rgb_enabled,
      self.t("tc_adv_str14"),
    )
    .clicked()
    {
      actions.push(GameAction::ToggleRgb);
    }

    let brightness_label = self.t("tc_adv_str7").to_string();
    let speed_label = self.t("tc_adv_str19").to_string();
    self.draw_slider_row(
      ui,
      429.0,
      &brightness_label,
      self.draft.brightness,
      0,
      4,
      &mut actions,
      true,
    );
    self.draw_slider_row(
      ui,
      484.0,
      &speed_label,
      self.draft.speed,
      1,
      4,
      &mut actions,
      false,
    );

    // Preset colour squares: the vendor palette is a 3x3 grid
    // (red/green/blue, magenta/dark-green/cyan, yellow/orange/white).
    for (index, preset) in PRESET_COLORS.iter().enumerate() {
      let col = index % 3;
      let row = index / 3;
      let rect = Rect::from_min_size(
        egui::pos2(680.0 + col as f32 * 27.0, 429.0 + row as f32 * 27.0),
        egui::vec2(22.0, 22.0),
      );
      ui.painter()
        .rect_filled(rect, 0.0, Color32::from_rgb(preset.r, preset.g, preset.b));
      let response = ui.interact(rect, Id::unique(("preset", index)), Sense::click());
      if response.clicked() {
        actions.push(GameAction::SetColor(*preset));
      }
    }

    // R/G/B numeric fields. The vendor puts the letter outside the box and
    // right-aligns the value in a simple one-pixel frame.
    {
      let mut rgb = [color.r, color.g, color.b];
      let labels = ["R", "G", "B"];
      let xs = [778.0, 844.0, 910.0];
      let label_xs = [766.0, 832.0, 898.0];
      for index in 0..3 {
        widgets::text(
          ui,
          egui::pos2(label_xs[index], 410.0),
          labels[index],
          12.0,
          widgets::TEXT,
        );
        let rect = Rect::from_min_size(egui::pos2(xs[index], 409.0), egui::vec2(42.0, 16.0));
        ui.painter()
          .rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 20));
        ui.painter().rect_stroke(
          rect,
          0.0,
          Stroke::new(1.0, Color32::from_rgb(190, 190, 190)),
          egui::StrokeKind::Inside,
        );
        widgets::text(
          ui,
          egui::pos2(
            rect.max.x - 3.0 - 8.0 * rgb[index].to_string().len() as f32,
            rect.min.y + 2.0,
          ),
          &rgb[index].to_string(),
          11.0,
          widgets::TEXT,
        );
        let response = ui.interact(
          rect,
          Id::unique(("rgb_field", index)),
          Sense::click_and_drag(),
        );
        if response.dragged() {
          let delta = ui.input(|i| i.pointer.delta().x);
          let value = (rgb[index] as f32 + delta * 0.5).round().clamp(0.0, 255.0) as u8;
          if value != rgb[index] {
            rgb[index] = value;
            actions.push(GameAction::SetColor(Rgb::new(rgb[0], rgb[1], rgb[2])));
          }
        }
        response.on_hover_text("Drag left/right to change");
      }
    }

    // Hue spectrum: the vendor reuses rcolor.png at native width as a
    // 156x66 picker inside the panel.
    let gradient_rect = Rect::from_min_size(egui::pos2(779.0, 429.0), egui::vec2(156.0, 66.0));
    paint_texture(
      ui.painter(),
      &self.assets.rcolor,
      gradient_rect,
      Color32::WHITE,
    );
    let gradient = ui.interact(
      gradient_rect,
      Id::unique("gradient"),
      Sense::click_and_drag(),
    );
    if gradient.dragged() || gradient.clicked() {
      if let Some(pointer) = gradient.interact_pointer_pos() {
        let x = ((pointer.x - gradient_rect.min.x) / gradient_rect.width()).clamp(0.0, 0.999);
        let y = ((pointer.y - gradient_rect.min.y) / gradient_rect.height()).clamp(0.0, 0.999);
        let px = (x * self.rcolor_size[0] as f32) as usize;
        let py = (y * self.rcolor_size[1] as f32) as usize;
        let index = (py * self.rcolor_size[0] + px) * 4;
        if index + 3 < self.rcolor_pixels.len() {
          actions.push(GameAction::SetColor(Rgb::new(
            self.rcolor_pixels[index],
            self.rcolor_pixels[index + 1],
            self.rcolor_pixels[index + 2],
          )));
        }
      }
    }

    // Status / device line, in the empty strip below the panel.
    if let Some(error) = &self.device_error {
      widgets::text(
        ui,
        egui::pos2(285.0, 537.0),
        &format!("{} {}", self.t("tc_msg15"), short_error(error)),
        11.0,
        Color32::from_rgb(230, 90, 90),
      );
    } else if self
      .status_until
      .map(|until| until > Instant::now())
      .unwrap_or(false)
    {
      widgets::text(
        ui,
        egui::pos2(285.0, 537.0),
        &self.status,
        11.0,
        widgets::TEXT_DIM,
      );
    }
    actions
  }

  #[allow(clippy::too_many_arguments)]
  fn draw_slider_row(
    &mut self,
    ui: &mut Ui,
    y: f32,
    label: &str,
    value: u8,
    min: u8,
    max: u8,
    actions: &mut Vec<GameAction>,
    brightness: bool,
  ) {
    widgets::text(ui, egui::pos2(329.0, y), label, 12.0, widgets::TEXT);
    let track = Rect::from_min_size(egui::pos2(404.0, y + 8.0), egui::vec2(195.0, 1.0));
    if let Some(new_value) = widgets::slider(ui, &self.assets, track, value, min, max) {
      actions.push(if brightness {
        GameAction::SetBrightness(new_value)
      } else {
        GameAction::SetSpeed(new_value)
      });
    }
    widgets::text(
      ui,
      egui::pos2(607.0, y),
      &value.to_string(),
      12.0,
      widgets::TEXT,
    );
    // Minus / plus hit areas under the track.
    let minus = Rect::from_min_size(egui::pos2(400.0, y + 24.0), egui::vec2(12.0, 14.0));
    let plus = Rect::from_min_size(egui::pos2(586.0, y + 24.0), egui::vec2(12.0, 14.0));
    for (rect, text, delta) in [(minus, "-", -1i16), (plus, "+", 1i16)] {
      widgets::text_centered(ui, rect, text, 13.0, widgets::TEXT);
      let response = ui.interact(
        rect,
        Id::unique(("step", rect.min.x.to_bits(), rect.min.y.to_bits())),
        Sense::click(),
      );
      if response.clicked() {
        let new_value = (value as i16 + delta).clamp(min as i16, max as i16) as u8;
        actions.push(if brightness {
          GameAction::SetBrightness(new_value)
        } else {
          GameAction::SetSpeed(new_value)
        });
      }
    }
  }

  fn draw_footer(&mut self, ui: &mut Ui) -> Vec<GameAction> {
    let mut actions = Vec::new();
    let export = Rect::from_min_size(egui::pos2(36.0, 560.0), egui::vec2(80.0, 28.0));
    let import = Rect::from_min_size(egui::pos2(119.0, 560.0), egui::vec2(98.0, 28.0));
    let restore = Rect::from_min_size(egui::pos2(220.0, 560.0), egui::vec2(80.0, 28.0));
    let ok = Rect::from_min_size(egui::pos2(435.0, 560.0), egui::vec2(75.0, 28.0));
    let cancel = Rect::from_min_size(egui::pos2(510.0, 560.0), egui::vec2(75.0, 28.0));
    let apply = Rect::from_min_size(egui::pos2(585.0, 560.0), egui::vec2(75.0, 28.0));

    if widgets::button(
      ui,
      &self.assets,
      export,
      self.t("tc_msg32"),
      ButtonKind::Wide,
    )
    .clicked()
    {
      actions.push(GameAction::Export);
    }
    let import_text = self.t("tc_msg33");
    let import_chars: Vec<char> = import_text.chars().collect();
    let import_label = if import_chars.len() > 11 {
      format!("{}...", import_chars[..11].iter().collect::<String>())
    } else {
      import_text.to_string()
    };
    if widgets::button(ui, &self.assets, import, &import_label, ButtonKind::Wide).clicked() {
      actions.push(GameAction::Import);
    }
    if widgets::button(
      ui,
      &self.assets,
      restore,
      self.t("tc_restore"),
      ButtonKind::Wide,
    )
    .clicked()
    {
      actions.push(GameAction::Restore);
    }
    if widgets::button(ui, &self.assets, ok, self.t("tc_ok"), ButtonKind::Normal).clicked() {
      actions.push(GameAction::Ok);
    }
    if widgets::button(
      ui,
      &self.assets,
      cancel,
      self.t("tc_cancel"),
      ButtonKind::Normal,
    )
    .clicked()
    {
      actions.push(GameAction::Cancel);
    }
    if widgets::button(
      ui,
      &self.assets,
      apply,
      self.t("tc_apply"),
      ButtonKind::Normal,
    )
    .clicked()
    {
      actions.push(GameAction::Apply);
    }

    let web_rect = Rect::from_min_size(egui::pos2(111.0, 600.0), egui::vec2(160.0, 18.0));
    let web = ui.interact(web_rect, Id::unique("web"), Sense::click());
    widgets::text(
      ui,
      egui::pos2(111.0, 601.0),
      self.t("tc_web"),
      11.0,
      if web.hovered() {
        widgets::ACCENT_BRIGHT
      } else {
        widgets::ACCENT
      },
    );
    if web.clicked() {
      actions.push(GameAction::OpenWeb);
    }
    actions
  }

  fn draw_mode_popup(&mut self, ui: &mut Ui) -> Option<GameAction> {
    if !self.mode_open {
      return None;
    }
    let rows: Vec<String> = Effect::ALL.iter().map(|e| self.mode_name(*e)).collect();
    // The list is taller than the space below the combo, so it opens
    // upwards like a standard Win32 combo popup.
    let rect = Rect::from_min_size(egui::pos2(404.0, 45.0), egui::vec2(169.0, 330.0));
    let selected = Effect::ALL
      .iter()
      .position(|e| Some(*e) == self.draft.effect().ok());
    let clicked = widgets::popup_list(ui, rect, &rows, selected);
    if let Some(index) = clicked {
      self.mode_open = false;
      return Some(GameAction::SelectEffect(Effect::ALL[index]));
    }
    // Click outside closes the popup. The caller consumes the click so it
    // does not also activate the control underneath.
    if ui.input(|i| i.pointer.primary_clicked()) {
      if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
        if !rect.contains(pos) {
          self.mode_open = false;
        }
      }
    }
    None
  }

  fn draw_lang_popup(&mut self, ui: &mut Ui) -> Option<GameAction> {
    if !self.lang_open {
      return None;
    }
    let rect = Rect::from_min_size(egui::pos2(800.0, 47.0), egui::vec2(94.0, 36.0));
    let rows = vec![
      Lang::English.label().to_string(),
      Lang::Chinese.label().to_string(),
    ];
    let selected = Some(match self.lang {
      Lang::English => 0,
      Lang::Chinese => 1,
    });
    let clicked = widgets::popup_list(ui, rect, &rows, selected);
    if let Some(index) = clicked {
      self.lang_open = false;
      return Some(GameAction::SetLang(if index == 0 {
        Lang::English
      } else {
        Lang::Chinese
      }));
    }
    if ui.input(|i| i.pointer.primary_clicked()) {
      if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
        if !rect.contains(pos) {
          self.lang_open = false;
        }
      }
    }
    None
  }

  fn draw_color_popup(&mut self, ui: &mut Ui) -> Option<GameAction> {
    if !self.color_open {
      return None;
    }
    let rect = Rect::from_min_size(egui::pos2(712.0, 391.0), egui::vec2(156.0, 112.0));
    paint_texture(ui.painter(), &self.assets.rcolor, rect, Color32::WHITE);
    let response = ui.interact(rect, Id::unique("rcolor_pick"), Sense::click());
    if response.clicked() {
      if let Some(pointer) = response.interact_pointer_pos() {
        let x = ((pointer.x - rect.min.x) / rect.width()).clamp(0.0, 0.999);
        let y = ((pointer.y - rect.min.y) / rect.height()).clamp(0.0, 0.999);
        let px = (x * self.rcolor_size[0] as f32) as usize;
        let py = (y * self.rcolor_size[1] as f32) as usize;
        let index = (py * self.rcolor_size[0] + px) * 4;
        if index + 3 < self.rcolor_pixels.len() {
          let color = Rgb::new(
            self.rcolor_pixels[index],
            self.rcolor_pixels[index + 1],
            self.rcolor_pixels[index + 2],
          );
          self.color_open = false;
          return Some(GameAction::SetColor(color));
        }
      }
    }
    if ui.input(|i| i.pointer.primary_clicked()) {
      if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
        if !rect.contains(pos) && !rect.expand(0.0).contains(pos) {
          self.color_open = false;
        }
      }
    }
    None
  }

  fn draw_file_dialog(&mut self, ui: &mut Ui) -> Option<GameAction> {
    let export_title = self.t("tc_msg32").to_string();
    let import_title = self.t("tc_msg33").to_string();
    let ok_label = self.t("tc_ok").to_string();
    let cancel_label = self.t("tc_cancel").to_string();
    let draft_json = serde_json::to_string_pretty(&self.draft).unwrap_or_default();
    let assets = &self.assets;
    let Some(dialog) = &mut self.file_dialog else {
      return None;
    };
    let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 640.0));
    ui.painter()
      .rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 150));
    let rect = Rect::from_center_size(egui::pos2(500.0, 320.0), egui::vec2(440.0, 140.0));
    ui.painter()
      .rect_filled(rect, 4.0, Color32::from_rgb(38, 38, 40));
    ui.painter().rect_stroke(
      rect,
      4.0,
      Stroke::new(1.0, Color32::from_rgb(110, 110, 110)),
      egui::StrokeKind::Inside,
    );
    let title = if dialog.mode == FileMode::Export {
      &export_title
    } else {
      &import_title
    };
    widgets::text(
      ui,
      egui::pos2(rect.min.x + 14.0, rect.min.y + 12.0),
      title,
      13.0,
      widgets::TEXT,
    );
    let path_rect = Rect::from_min_size(
      egui::pos2(rect.min.x + 14.0, rect.min.y + 38.0),
      egui::vec2(rect.width() - 28.0, 20.0),
    );
    ui.painter()
      .rect_filled(path_rect, 0.0, Color32::from_rgb(20, 20, 20));
    ui.painter().rect_stroke(
      path_rect,
      0.0,
      Stroke::new(1.0, Color32::from_rgb(190, 190, 190)),
      egui::StrokeKind::Inside,
    );
    ui.scope(|ui| {
      let visuals = &mut ui.style_mut().visuals;
      visuals.override_text_color = Some(widgets::TEXT);
      visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
      ui.put(
        path_rect.shrink2(egui::vec2(3.0, 2.0)),
        egui::TextEdit::singleline(&mut dialog.path).frame(egui::Frame::NONE),
      );
    });
    if let Some(error) = &dialog.error {
      widgets::text(
        ui,
        egui::pos2(rect.min.x + 14.0, rect.min.y + 62.0),
        error,
        11.0,
        Color32::from_rgb(230, 90, 90),
      );
    }
    let ok = Rect::from_min_size(
      egui::pos2(rect.center().x - 80.0, rect.max.y - 32.0),
      egui::vec2(70.0, 24.0),
    );
    let cancel = Rect::from_min_size(
      egui::pos2(rect.center().x + 10.0, rect.max.y - 32.0),
      egui::vec2(70.0, 24.0),
    );
    let mut ok_clicked = false;
    let mut cancel_clicked = false;
    if widgets::button(ui, assets, ok, &ok_label, ButtonKind::Normal).clicked() {
      ok_clicked = true;
    }
    if widgets::button(ui, assets, cancel, &cancel_label, ButtonKind::Normal).clicked() {
      cancel_clicked = true;
    }
    let mode = dialog.mode;
    let path = PathBuf::from(dialog.path.trim());
    if cancel_clicked {
      self.file_dialog = None;
      return None;
    }
    if !ok_clicked {
      return None;
    }
    match mode {
      FileMode::Export => match std::fs::write(&path, draft_json) {
        Ok(()) => {
          self.file_dialog = None;
          self.set_status(format!("{} {}", export_title, path.display()), 3);
        }
        Err(error) => {
          if let Some(dialog) = &mut self.file_dialog {
            dialog.error = Some(format!("{error}"));
          }
        }
      },
      FileMode::Import => match std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Profile>(&text).ok())
      {
        Some(profile) => {
          self.draft = profile;
          self.file_dialog = None;
          self.mark_dirty();
          return Some(GameAction::Apply);
        }
        None => {
          if let Some(dialog) = &mut self.file_dialog {
            dialog.error = Some(format!("{}: {}", import_title, path.display()));
          }
        }
      },
    }
    None
  }

  fn draw_restore_dialog(&mut self, ui: &mut Ui) -> Option<GameAction> {
    if !self.confirm_restore {
      return None;
    }
    let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 640.0));
    ui.painter()
      .rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 150));
    let rect = Rect::from_center_size(egui::pos2(500.0, 320.0), egui::vec2(430.0, 110.0));
    ui.painter()
      .rect_filled(rect, 4.0, Color32::from_rgb(38, 38, 40));
    widgets::text(
      ui,
      egui::pos2(rect.min.x + 16.0, rect.min.y + 18.0),
      self.t("tc_msg5"),
      13.0,
      widgets::TEXT,
    );
    let ok = Rect::from_min_size(
      egui::pos2(rect.center().x - 80.0, rect.max.y - 34.0),
      egui::vec2(70.0, 24.0),
    );
    let cancel = Rect::from_min_size(
      egui::pos2(rect.center().x + 10.0, rect.max.y - 34.0),
      egui::vec2(70.0, 24.0),
    );
    if widgets::button(ui, &self.assets, ok, self.t("tc_ok"), ButtonKind::Normal).clicked() {
      return Some(GameAction::ConfirmRestore(true));
    }
    if widgets::button(
      ui,
      &self.assets,
      cancel,
      self.t("tc_cancel"),
      ButtonKind::Normal,
    )
    .clicked()
    {
      return Some(GameAction::ConfirmRestore(false));
    }
    None
  }
}

impl eframe::App for App {
  fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    [0.0, 0.0, 0.0, 0.0]
  }

  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    if self.dirty && self.last_change.elapsed() >= Duration::from_millis(90) {
      self.dirty = false;
      self.apply_draft();
    }

    let mut actions = Vec::new();
    ui.set_min_size(egui::vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]));
    paint_texture(
      ui.painter(),
      &self.assets.main_bg,
      Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 640.0)),
      Color32::WHITE,
    );
    actions.extend(self.draw_title_bar(ui));
    actions.extend(self.draw_profile_bar(ui));
    let colors = (self.draft.effect().ok() == Some(Effect::SelfDefine) && self.rgb_enabled)
      .then(|| self.profile_colors());
    if let Some(click) = keyboard::draw(ui, &self.assets, colors.as_ref(), self.selected_key) {
      actions.push(GameAction::SelectKey(click.index, click.right));
    }
    let hint = self.t("tc_msg23").to_string();
    keyboard::draw_hint(ui, &hint);
    actions.extend(self.draw_light_panel(ui));
    actions.extend(self.draw_footer(ui));
    if let Some(action) = self.draw_mode_popup(ui) {
      actions.push(action);
    }
    if let Some(action) = self.draw_lang_popup(ui) {
      actions.push(action);
    }
    if let Some(action) = self.draw_color_popup(ui) {
      actions.push(action);
    }
    if let Some(action) = self.draw_file_dialog(ui) {
      actions.push(action);
    }
    if let Some(action) = self.draw_restore_dialog(ui) {
      actions.push(action);
    }

    for action in actions {
      self.handle_action(action, ui.ctx());
    }

    if self.macro_editor.open {
      self.macro_editor.show(
        ui.ctx(),
        &self.assets,
        self.lang,
        &self.i18n_en,
        &self.i18n_sc,
      );
    }
    ui.ctx().request_repaint_after(Duration::from_millis(16));
  }
}

fn install_fonts(ctx: &egui::Context) {
  let mut fonts = egui::FontDefinitions::default();
  fonts.font_data.insert(
    "dejavu".to_owned(),
    egui::FontData::from_static(include_bytes!("../../assets/gui/DejaVuSans.ttf")).into(),
  );
  fonts
    .families
    .get_mut(&egui::FontFamily::Proportional)
    .expect("proportional family")
    .insert(0, "dejavu".to_owned());
  ctx.set_fonts(fonts);

  let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
  style.text_styles = [
    (
      egui::TextStyle::Body,
      FontId::new(12.0, egui::FontFamily::Proportional),
    ),
    (
      egui::TextStyle::Button,
      FontId::new(12.0, egui::FontFamily::Proportional),
    ),
    (
      egui::TextStyle::Small,
      FontId::new(11.0, egui::FontFamily::Proportional),
    ),
    (
      egui::TextStyle::Heading,
      FontId::new(16.0, egui::FontFamily::Proportional),
    ),
    (
      egui::TextStyle::Monospace,
      FontId::new(11.0, egui::FontFamily::Monospace),
    ),
  ]
  .into();
  style.visuals = egui::Visuals::dark();
  ctx.set_style_of(egui::Theme::Dark, style.clone());
  ctx.set_style_of(egui::Theme::Light, style);
  ctx.set_theme(egui::ThemePreference::Dark);
}

fn load_profiles() -> [Profile; 3] {
  let default = Profile::new(Effect::FixedOn, Rgb::new(255, 0, 0), 3, 3);
  let mut profiles = [default.clone(), default.clone(), default];
  for (index, slot) in profiles.iter_mut().enumerate() {
    let name = format!("Profile {}", index + 1);
    if let Ok(profile) = profile::load_named(&name) {
      *slot = profile;
    }
  }
  profiles
}

fn default_export_path() -> String {
  profile::config_dir()
    .map(|dir| dir.join("export.json").to_string_lossy().into_owned())
    .unwrap_or_else(|_| "export.json".to_string())
}

fn short_error(error: &str) -> String {
  let line = error.lines().next().unwrap_or(error);
  if line.len() > 90 {
    format!("{}...", &line[..90])
  } else {
    line.to_string()
  }
}
