//! Macro Editor window.
//!
//! This reproduces the vendor window (560x360, macro_nr.png) and stores macros
//! as JSON. The vendor's macro *upload* format is not part of the current port,
//! so the editor manages host-side macros; recording captures key events
//! delivered to this window.

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{
  self, Color32, Id, Rect, Sense, Stroke, Ui, ViewportBuilder, ViewportCommand, ViewportId,
};
use serde::{Deserialize, Serialize};

use crate::profile;

use super::assets::{paint_texture, Assets};
use super::i18n::{Lang, Texts};
use super::widgets::{self, ButtonKind};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MacroEvent {
  /// "down", "up" or "delay".
  pub kind: String,
  pub key: String,
  pub delay_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroEntry {
  pub name: String,
  #[serde(default)]
  pub folder: bool,
  #[serde(default)]
  pub events: Vec<MacroEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptAction {
  NewMacro,
  NewFolder,
  Rename(usize),
}

struct Prompt {
  action: PromptAction,
  value: String,
}

pub struct MacroEditor {
  pub open: bool,
  macros: Vec<MacroEntry>,
  selected: Option<usize>,
  selected_event: Option<usize>,
  recording: bool,
  auto_delay: bool,
  cycle_key: bool,
  cycle_times: bool,
  cycles_text: String,
  insert_open: bool,
  prompt: Option<Prompt>,
  context: Option<(Option<usize>, egui::Pos2)>,
  status: String,
  last_record: Instant,
}

impl MacroEditor {
  pub fn load() -> Self {
    Self {
      open: false,
      macros: load_macros(),
      selected: None,
      selected_event: None,
      recording: false,
      auto_delay: true,
      cycle_key: false,
      cycle_times: false,
      cycles_text: "1".to_string(),
      insert_open: false,
      prompt: None,
      context: None,
      status: String::new(),
      last_record: Instant::now(),
    }
  }

  fn save(&self) {
    let Some(path) = macros_path() else {
      return;
    };
    if let Some(parent) = path.parent() {
      let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&self.macros) {
      let _ = std::fs::write(path, text + "\n");
    }
  }

  pub fn show(
    &mut self,
    ctx: &egui::Context,
    assets: &Assets,
    lang: Lang,
    en: &super::i18n::I18n,
    sc: &super::i18n::I18n,
  ) {
    let id = ViewportId::from_hash_of("macro-editor");
    let builder = ViewportBuilder::default()
      .with_inner_size([560.0, 360.0])
      .with_decorations(false)
      .with_transparent(true)
      .with_resizable(false)
      .with_title("Macro Editor");
    let mut close = false;
    ctx.show_viewport_immediate(id, builder, |ui, _class| {
      ui.set_min_size(egui::vec2(560.0, 360.0));
      paint_texture(
        ui.painter(),
        &assets.macro_bg,
        Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(560.0, 360.0)),
        Color32::WHITE,
      );
      let texts = Texts::new(lang, en, sc);

      let drag = ui.interact(
        Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(520.0, 30.0)),
        Id::unique("macro_drag"),
        Sense::click_and_drag(),
      );
      if drag.dragged() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
      }
      let close_rect = Rect::from_min_size(egui::pos2(527.0, 9.0), egui::vec2(16.0, 16.0));
      let close_resp = ui.interact(close_rect, Id::unique("macro_close"), Sense::click());
      ui.painter().rect_filled(
        close_rect,
        0.0,
        if close_resp.hovered() {
          Color32::from_rgb(255, 80, 80)
        } else {
          Color32::from_rgb(200, 0, 0)
        },
      );
      let c = close_rect.center();
      let stroke = Stroke::new(1.5, Color32::WHITE);
      ui.painter().line_segment(
        [c + egui::vec2(-4.0, -4.0), c + egui::vec2(4.0, 4.0)],
        stroke,
      );
      ui.painter().line_segment(
        [c + egui::vec2(-4.0, 4.0), c + egui::vec2(4.0, -4.0)],
        stroke,
      );
      if close_resp.clicked() {
        close = true;
      }

      widgets::text(
        ui,
        egui::pos2(32.0, 60.0),
        texts.t("tc_macro_list"),
        12.0,
        widgets::TEXT,
      );
      widgets::text(
        ui,
        egui::pos2(191.0, 60.0),
        texts.t("tc_macro_key_list"),
        12.0,
        widgets::TEXT,
      );

      self.record_input(ui.ctx(), &texts);

      let mut actions: Vec<MacroUiAction> = Vec::new();
      self.draw_macro_list(ui, &texts);
      self.draw_event_list(ui, &texts);
      self.draw_controls(ui, assets, &texts, &mut actions);
      self.draw_prompt(ui, assets, &texts, &mut actions);
      self.draw_context_menu(ui, &texts, &mut actions);
      self.process(actions, &texts);
    });
    if close {
      self.open = false;
      self.recording = false;
    }
  }

  fn draw_macro_list(&mut self, ui: &mut Ui, texts: &Texts) {
    let rect = Rect::from_min_size(egui::pos2(30.0, 78.0), egui::vec2(152.0, 247.0));
    ui.painter().rect_filled(rect, 0.0, widgets::LIST_BG);
    ui.painter().rect_stroke(
      rect,
      0.0,
      Stroke::new(1.0, Color32::from_rgb(90, 90, 90)),
      egui::StrokeKind::Inside,
    );
    let background = ui.interact(rect, Id::unique("macro_list_bg"), Sense::click());
    if background.secondary_clicked() {
      if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
        self.context = Some((None, pointer));
      }
    }
    let mut y = rect.min.y + 2.0;
    for (index, entry) in self.macros.iter().enumerate() {
      if y + 18.0 > rect.max.y {
        break;
      }
      let row = Rect::from_min_size(
        egui::pos2(rect.min.x + 2.0, y),
        egui::vec2(rect.width() - 4.0, 18.0),
      );
      let response = ui.interact(row, Id::unique(("macro_row", index)), Sense::click());
      if self.selected == Some(index) {
        ui.painter()
          .rect_filled(row, 0.0, Color32::from_rgb(210, 220, 245));
      } else if response.hovered() {
        ui.painter()
          .rect_filled(row, 0.0, Color32::from_rgb(235, 235, 245));
      }
      let prefix = if entry.folder { "[+] " } else { "" };
      widgets::text(
        ui,
        egui::pos2(row.min.x + 4.0, row.min.y + 2.0),
        &format!("{prefix}{}", entry.name),
        11.0,
        widgets::LIST_TEXT,
      );
      if response.clicked() {
        self.selected = Some(index);
        self.selected_event = None;
      }
      if response.secondary_clicked() {
        self.context = Some((Some(index), row.center()));
      }
      y += 18.0;
    }
    widgets::text(
      ui,
      egui::pos2(32.0, 330.0),
      texts.t("tc_macro_msg9"),
      11.0,
      widgets::TEXT_DIM,
    );
  }

  fn draw_event_list(&mut self, ui: &mut Ui, texts: &Texts) {
    let rect = Rect::from_min_size(egui::pos2(191.0, 78.0), egui::vec2(129.0, 247.0));
    ui.painter().rect_filled(rect, 0.0, widgets::LIST_BG);
    ui.painter().rect_stroke(
      rect,
      0.0,
      Stroke::new(1.0, Color32::from_rgb(90, 90, 90)),
      egui::StrokeKind::Inside,
    );
    let Some(index) = self.selected else {
      return;
    };
    let Some(entry) = self.macros.get(index) else {
      return;
    };
    let mut y = rect.min.y + 2.0;
    for (event_index, event) in entry.events.iter().enumerate() {
      if y + 16.0 > rect.max.y {
        break;
      }
      let row = Rect::from_min_size(
        egui::pos2(rect.min.x + 2.0, y),
        egui::vec2(rect.width() - 4.0, 16.0),
      );
      let response = ui.interact(
        row,
        Id::unique(("macro_event", index, event_index)),
        Sense::click(),
      );
      if self.selected_event == Some(event_index) {
        ui.painter()
          .rect_filled(row, 0.0, Color32::from_rgb(210, 220, 245));
      } else if response.hovered() {
        ui.painter()
          .rect_filled(row, 0.0, Color32::from_rgb(235, 235, 245));
      }
      let label = match event.kind.as_str() {
        "down" => format!("{}: {}", texts.t("tc_macro_msg14"), event.key),
        "up" => format!("{}: {}", texts.t("tc_macro_msg15"), event.key),
        _ => format!("{}: {} ms", texts.t("tc_macro_msg16"), event.delay_ms),
      };
      widgets::text(
        ui,
        egui::pos2(row.min.x + 4.0, row.min.y + 1.0),
        &label,
        11.0,
        widgets::LIST_TEXT,
      );
      if response.clicked() {
        self.selected_event = Some(event_index);
      }
      y += 16.0;
    }
  }

  fn draw_controls(
    &mut self,
    ui: &mut Ui,
    assets: &Assets,
    texts: &Texts,
    actions: &mut Vec<MacroUiAction>,
  ) {
    let record_rect = Rect::from_min_size(egui::pos2(350.0, 80.0), egui::vec2(112.0, 23.0));
    let texture = if self.recording {
      &assets.recbtn_dn
    } else {
      &assets.recbtn_nr
    };
    paint_texture(ui.painter(), texture, record_rect, Color32::WHITE);
    let record = ui.interact(record_rect, Id::unique("record"), Sense::click());
    let label = if self.recording {
      texts.t("tc_macro_stop")
    } else {
      texts.t("tc_macro_start")
    };
    widgets::text_centered(ui, record_rect, label, 12.0, widgets::TEXT);
    if record.clicked() {
      actions.push(MacroUiAction::ToggleRecord);
    }

    let auto_rect = Rect::from_min_size(egui::pos2(350.0, 117.0), egui::vec2(13.0, 12.0));
    if widgets::checkbox_size(
      ui,
      assets,
      auto_rect,
      self.auto_delay,
      texts.t("tc_macro_auto"),
      11.0,
    )
    .clicked()
    {
      self.auto_delay = !self.auto_delay;
    }
    let cycle_key_rect = Rect::from_min_size(egui::pos2(350.0, 138.0), egui::vec2(13.0, 12.0));
    if widgets::checkbox_size(
      ui,
      assets,
      cycle_key_rect,
      self.cycle_key,
      texts.t("tc_macro_loop2"),
      11.0,
    )
    .clicked()
    {
      self.cycle_key = !self.cycle_key;
    }
    let cycle_times_rect = Rect::from_min_size(egui::pos2(350.0, 159.0), egui::vec2(13.0, 12.0));
    if widgets::checkbox_size(
      ui,
      assets,
      cycle_times_rect,
      self.cycle_times,
      texts.t("tc_macro_loop1"),
      11.0,
    )
    .clicked()
    {
      self.cycle_times = !self.cycle_times;
    }

    let cycles_rect = Rect::from_min_size(egui::pos2(350.0, 180.0), egui::vec2(99.0, 19.0));
    ui.painter()
      .rect_filled(cycles_rect, 0.0, Color32::from_rgb(20, 20, 20));
    ui.painter().rect_stroke(
      cycles_rect,
      0.0,
      Stroke::new(1.0, Color32::from_rgb(190, 190, 190)),
      egui::StrokeKind::Inside,
    );
    ui.scope(|ui| {
      let visuals = &mut ui.style_mut().visuals;
      visuals.override_text_color = Some(widgets::TEXT);
      visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
      ui.put(
        cycles_rect.shrink2(egui::vec2(3.0, 2.0)),
        egui::TextEdit::singleline(&mut self.cycles_text)
          .frame(egui::Frame::NONE)
          .char_limit(5),
      );
    });
    self.cycles_text.retain(|c| c.is_ascii_digit());

    widgets::text(
      ui,
      egui::pos2(350.0, 220.0),
      texts.t("tc_macro_loop"),
      12.0,
      widgets::TEXT,
    );
    let insert_rect = Rect::from_min_size(egui::pos2(350.0, 238.0), egui::vec2(172.0, 19.0));
    if widgets::combo(ui, assets, insert_rect, "", self.insert_open).clicked() {
      self.insert_open = !self.insert_open;
    }
    if self.insert_open {
      let popup = Rect::from_min_size(egui::pos2(350.0, 196.0), egui::vec2(172.0, 63.0));
      let rows = vec![
        texts.t("tc_macro_msg14").to_string(),
        texts.t("tc_macro_msg15").to_string(),
        texts.t("tc_macro_msg16").to_string(),
      ];
      if let Some(index) = widgets::popup_list(ui, popup, &rows, None) {
        self.insert_open = false;
        actions.push(MacroUiAction::Insert(index));
      }
    }

    if !self.status.is_empty() {
      widgets::text(
        ui,
        egui::pos2(350.0, 266.0),
        &self.status,
        10.0,
        Color32::from_rgb(230, 90, 90),
      );
    }
  }

  fn draw_prompt(
    &mut self,
    ui: &mut Ui,
    assets: &Assets,
    texts: &Texts,
    actions: &mut Vec<MacroUiAction>,
  ) {
    let Some(prompt) = &mut self.prompt else {
      return;
    };
    let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(560.0, 360.0));
    ui.painter()
      .rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 140));
    let rect = Rect::from_center_size(egui::pos2(280.0, 180.0), egui::vec2(300.0, 100.0));
    ui.painter()
      .rect_filled(rect, 3.0, Color32::from_rgb(40, 40, 42));
    let title = match prompt.action {
      PromptAction::NewMacro => texts.t("tc_macro_msg5"),
      PromptAction::NewFolder => texts.t("tc_macro_msg6"),
      PromptAction::Rename(_) => texts.t("tc_macro_msg7"),
    };
    widgets::text(
      ui,
      egui::pos2(rect.min.x + 12.0, rect.min.y + 12.0),
      title,
      12.0,
      widgets::TEXT,
    );
    let field = Rect::from_min_size(
      egui::pos2(rect.min.x + 12.0, rect.min.y + 34.0),
      egui::vec2(rect.width() - 24.0, 20.0),
    );
    ui.painter()
      .rect_filled(field, 0.0, Color32::from_rgb(20, 20, 20));
    ui.painter().rect_stroke(
      field,
      0.0,
      Stroke::new(1.0, Color32::from_rgb(190, 190, 190)),
      egui::StrokeKind::Inside,
    );
    ui.scope(|ui| {
      let visuals = &mut ui.style_mut().visuals;
      visuals.override_text_color = Some(widgets::TEXT);
      visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
      ui.put(
        field.shrink2(egui::vec2(3.0, 2.0)),
        egui::TextEdit::singleline(&mut prompt.value).frame(egui::Frame::NONE),
      );
    });
    let ok = Rect::from_min_size(
      egui::pos2(rect.center().x - 75.0, rect.max.y - 30.0),
      egui::vec2(65.0, 22.0),
    );
    let cancel = Rect::from_min_size(
      egui::pos2(rect.center().x + 10.0, rect.max.y - 30.0),
      egui::vec2(65.0, 22.0),
    );
    if widgets::button(ui, assets, ok, texts.t("tc_ok"), ButtonKind::Normal).clicked() {
      actions.push(MacroUiAction::PromptOk);
    }
    if widgets::button(ui, assets, cancel, texts.t("tc_cancel"), ButtonKind::Normal).clicked() {
      self.prompt = None;
    }
  }

  fn draw_context_menu(&mut self, ui: &mut Ui, texts: &Texts, actions: &mut Vec<MacroUiAction>) {
    let Some((index, pos)) = self.context else {
      return;
    };
    let mut rows = vec![
      texts.t("tc_macro_msg5").to_string(),
      texts.t("tc_macro_msg6").to_string(),
    ];
    if index.is_some() {
      rows.push(texts.t("tc_macro_msg20").to_string());
      rows.push(texts.t("tc_macro_msg10").to_string());
    }
    let rect = Rect::from_min_size(pos, egui::vec2(140.0, rows.len() as f32 * 24.0));
    if let Some(row) = widgets::popup_list(ui, rect, &rows, None) {
      self.context = None;
      match row {
        0 => {
          self.prompt = Some(Prompt {
            action: PromptAction::NewMacro,
            value: format!("Macro {}", self.macros.len() + 1),
          });
        }
        1 => {
          self.prompt = Some(Prompt {
            action: PromptAction::NewFolder,
            value: "Folder".to_string(),
          });
        }
        2 => {
          if let Some(index) = index {
            if let Some(entry) = self.macros.get(index) {
              self.prompt = Some(Prompt {
                action: PromptAction::Rename(index),
                value: entry.name.clone(),
              });
            }
          }
        }
        _ => {
          if let Some(index) = index {
            actions.push(MacroUiAction::Delete(index));
          }
        }
      }
    }
    if ui.input(|i| i.pointer.primary_clicked()) {
      if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
        if !rect.contains(pointer) {
          self.context = None;
        }
      }
    }
  }

  fn process(&mut self, actions: Vec<MacroUiAction>, texts: &Texts) {
    for action in actions {
      match action {
        MacroUiAction::ToggleRecord => {
          if self.recording {
            self.recording = false;
          } else if self.selected.is_some() {
            self.recording = true;
            self.last_record = Instant::now();
          } else {
            self.status = texts.t("tc_macro_msg1").to_string();
          }
        }
        MacroUiAction::Insert(index) => {
          if let Some(selected) = self.selected {
            if let Some(entry) = self.macros.get_mut(selected) {
              let event = match index {
                0 => MacroEvent {
                  kind: "down".into(),
                  key: "A".into(),
                  delay_ms: 0,
                },
                1 => MacroEvent {
                  kind: "up".into(),
                  key: "A".into(),
                  delay_ms: 0,
                },
                _ => MacroEvent {
                  kind: "delay".into(),
                  key: String::new(),
                  delay_ms: 100,
                },
              };
              entry.events.push(event);
              self.save();
            }
          }
        }
        MacroUiAction::Delete(index) => {
          if index < self.macros.len() {
            self.macros.remove(index);
            if self.selected == Some(index) {
              self.selected = None;
              self.selected_event = None;
            }
            self.save();
          }
        }
        MacroUiAction::PromptOk => {
          if let Some(prompt) = self.prompt.take() {
            let name = prompt.value.trim().to_string();
            if name.is_empty() {
              continue;
            }
            match prompt.action {
              PromptAction::NewMacro => {
                self.macros.push(MacroEntry {
                  name,
                  folder: false,
                  events: Vec::new(),
                });
                self.selected = Some(self.macros.len() - 1);
              }
              PromptAction::NewFolder => {
                self.macros.push(MacroEntry {
                  name,
                  folder: true,
                  events: Vec::new(),
                });
                self.selected = Some(self.macros.len() - 1);
              }
              PromptAction::Rename(index) => {
                if let Some(entry) = self.macros.get_mut(index) {
                  entry.name = name;
                }
              }
            }
            self.save();
          }
        }
      }
    }
  }

  fn record_input(&mut self, ctx: &egui::Context, texts: &Texts) {
    if !self.recording {
      return;
    }
    let Some(selected) = self.selected else {
      self.recording = false;
      self.status = texts.t("tc_macro_msg1").to_string();
      return;
    };
    let events = ctx.input(|input| input.events.clone());
    if events.is_empty() {
      return;
    }
    let mut new_events = Vec::new();
    let now = Instant::now();
    if self.auto_delay {
      let delay = now.duration_since(self.last_record).as_millis() as u32;
      if delay >= 10 {
        new_events.push(MacroEvent {
          kind: "delay".into(),
          key: String::new(),
          delay_ms: delay,
        });
      }
    }
    for event in events {
      match event {
        egui::Event::Key {
          key,
          pressed,
          repeat,
          ..
        } if !repeat => {
          new_events.push(MacroEvent {
            kind: if pressed { "down".into() } else { "up".into() },
            key: format!("{key:?}"),
            delay_ms: 0,
          });
        }
        egui::Event::Text(text) => {
          new_events.push(MacroEvent {
            kind: "down".into(),
            key: text,
            delay_ms: 0,
          });
        }
        _ => {}
      }
    }
    if !new_events.is_empty() {
      if let Some(entry) = self.macros.get_mut(selected) {
        entry.events.extend(new_events);
        self.save();
      }
      self.last_record = now;
    }
  }
}

enum MacroUiAction {
  ToggleRecord,
  Insert(usize),
  Delete(usize),
  PromptOk,
}

fn macros_path() -> Option<PathBuf> {
  profile::config_dir()
    .ok()
    .map(|dir| dir.join("macros.json"))
}

fn load_macros() -> Vec<MacroEntry> {
  let Some(path) = macros_path() else {
    return Vec::new();
  };
  let Ok(text) = std::fs::read_to_string(path) else {
    return Vec::new();
  };
  serde_json::from_str(&text).unwrap_or_default()
}
