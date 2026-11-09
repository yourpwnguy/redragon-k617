//! Translation tables.
//!
//! The vendor ships the same key/value XML in `Text/en` and `Text/sc`. We
//! embed UTF-8 copies (the originals are UTF-16) and parse them at startup so
//! the language dropdown is a real switch instead of a decorative control.

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
  English,
  Chinese,
}

impl Lang {
  pub fn label(self) -> &'static str {
    match self {
      Lang::English => "English",
      Lang::Chinese => "简体中文",
    }
  }
}

pub struct I18n {
  map: HashMap<String, String>,
}

impl I18n {
  pub fn load(lang: Lang) -> Self {
    let xml = match lang {
      Lang::English => include_str!("../../assets/gui/text_en.xml"),
      Lang::Chinese => include_str!("../../assets/gui/text_sc.xml"),
    };
    Self { map: parse(xml) }
  }

  /// Look up a label. Missing keys return an empty string; callers that need
  /// a visible fallback should use [`Texts::t`].
  pub fn t<'a>(&'a self, key: &str) -> &'a str {
    self.map.get(key).map(String::as_str).unwrap_or("")
  }

  pub fn has(&self, key: &str) -> bool {
    self.map.contains_key(key)
  }
}

/// Language-aware view over both tables, with English as the fallback.
pub struct Texts<'a> {
  pub lang: Lang,
  pub en: &'a I18n,
  pub sc: &'a I18n,
}

impl<'a> Texts<'a> {
  pub fn new(lang: Lang, en: &'a I18n, sc: &'a I18n) -> Self {
    Self { lang, en, sc }
  }

  pub fn t(&self, key: &str) -> &'a str {
    let primary = match self.lang {
      Lang::English => self.en,
      Lang::Chinese => self.sc,
    };
    if primary.has(key) {
      primary.t(key)
    } else {
      self.en.t(key)
    }
  }
}

fn parse(xml: &str) -> HashMap<String, String> {
  let mut map = HashMap::new();
  let Ok(doc) = roxmltree::Document::parse(xml) else {
    return map;
  };
  for node in doc.descendants().filter(|n| n.is_element()) {
    let Some(text) = node.text() else {
      continue;
    };
    let value = text.trim();
    // The first occurrence wins; nested <root>/<config> wrappers are
    // ignored because they are not leaf labels.
    map
      .entry(node.tag_name().name().to_string())
      .or_insert_with(|| value.to_string());
  }
  map
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn loads_english_and_chinese_labels() {
    let en = I18n::load(Lang::English);
    assert_eq!(en.t("tc_msg1"), "Light");
    assert_eq!(en.t("tc_led_mode21"), "Self-define");
    let sc = I18n::load(Lang::Chinese);
    assert!(sc.has("tc_msg1"));
    assert_ne!(sc.t("tc_msg1"), "");
  }
}
