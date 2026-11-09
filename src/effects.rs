//! The 22 firmware lighting modes exposed by the official Windows software.
//!
//! The mode number is the byte the keyboard firmware expects in the commit
//! frame. It is not an arbitrary value: these are exactly the indices the
//! vendor app uses (`tc_led_mode1` .. `tc_led_mode22` in Text/en/text.xml),
//! so keeping this enum numbered the same way is what makes the port 1:1.

/// A firmware-native lighting effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Effect {
  FixedOn = 1,
  Respire = 2,
  Rainbow = 3,
  FlashAway = 4,
  Raindrops = 5,
  RainbowWheel = 6,
  RipplesShining = 7,
  StarsTwinkle = 8,
  ShadowDisappear = 9,
  RetroSnake = 10,
  NeonStream = 11,
  Reaction = 12,
  SineWave = 13,
  RetinueScanning = 14,
  RotatingWindmill = 15,
  ColorfulWaterfall = 16,
  Blossoming = 17,
  RotatingStorm = 18,
  Collision = 19,
  Perfect = 20,
  SelfDefine = 21,
  Off = 22,
}

impl Effect {
  pub const ALL: [Effect; 22] = [
    Effect::FixedOn,
    Effect::Respire,
    Effect::Rainbow,
    Effect::FlashAway,
    Effect::Raindrops,
    Effect::RainbowWheel,
    Effect::RipplesShining,
    Effect::StarsTwinkle,
    Effect::ShadowDisappear,
    Effect::RetroSnake,
    Effect::NeonStream,
    Effect::Reaction,
    Effect::SineWave,
    Effect::RetinueScanning,
    Effect::RotatingWindmill,
    Effect::ColorfulWaterfall,
    Effect::Blossoming,
    Effect::RotatingStorm,
    Effect::Collision,
    Effect::Perfect,
    Effect::SelfDefine,
    Effect::Off,
  ];

  pub fn number(self) -> u8 {
    self as u8
  }

  /// Canonical names use the vendor's own terminology. Aliases below are for
  /// convenience because users normally know effects by their UI labels.
  pub fn name(self) -> &'static str {
    match self {
      Effect::FixedOn => "fixed-on",
      Effect::Respire => "respire",
      Effect::Rainbow => "rainbow",
      Effect::FlashAway => "flash-away",
      Effect::Raindrops => "raindrops",
      Effect::RainbowWheel => "rainbow-wheel",
      Effect::RipplesShining => "ripples-shining",
      Effect::StarsTwinkle => "stars-twinkle",
      Effect::ShadowDisappear => "shadow-disappear",
      Effect::RetroSnake => "retro-snake",
      Effect::NeonStream => "neon-stream",
      Effect::Reaction => "reaction",
      Effect::SineWave => "sine-wave",
      Effect::RetinueScanning => "retinue-scanning",
      Effect::RotatingWindmill => "rotating-windmill",
      Effect::ColorfulWaterfall => "colorful-waterfall",
      Effect::Blossoming => "blossoming",
      Effect::RotatingStorm => "rotating-storm",
      Effect::Collision => "collision",
      Effect::Perfect => "perfect",
      Effect::SelfDefine => "self-define",
      Effect::Off => "off",
    }
  }

  /// Parse a user supplied name. Matching is case-insensitive and ignores
  /// spaces, underscores and hyphens, so "Rainbow Wheel", "rainbow-wheel"
  /// and "rainbow_wheel" all work.
  pub fn parse(input: &str) -> Option<Effect> {
    let normalized = normalize(input);
    Effect::ALL.into_iter().find(|e| {
      normalize(e.name()) == normalized || e.aliases().iter().any(|a| normalize(a) == normalized)
    })
  }

  fn aliases(self) -> &'static [&'static str] {
    match self {
      Effect::FixedOn => &["static", "fixed", "fixed on", "solid"],
      Effect::Respire => &["breathe", "breathing", "breath"],
      Effect::FlashAway => &["flash", "blink"],
      Effect::RainbowWheel => &["wheel", "rainbow wheel"],
      Effect::RipplesShining => &["ripples", "ripple"],
      Effect::StarsTwinkle => &["stars", "twinkle"],
      Effect::ShadowDisappear => &["shadow"],
      Effect::RetroSnake => &["snake"],
      Effect::NeonStream => &["neon"],
      Effect::Reaction => &["reactive"],
      Effect::SineWave => &["sine", "wave"],
      Effect::RetinueScanning => &["scanning", "scan"],
      Effect::RotatingWindmill => &["windmill"],
      Effect::ColorfulWaterfall => &["waterfall"],
      Effect::RotatingStorm => &["storm"],
      Effect::SelfDefine => &["custom", "per-key", "perkey"],
      Effect::Off => &["none", "dark"],
      _ => &[],
    }
  }

  /// The vendor UI only exposes a base color for some effects. We still let
  /// users set one everywhere; the firmware ignores it where it has no
  /// meaning, which matches the Windows app.
  pub fn supports_color(self) -> bool {
    !matches!(self, Effect::Rainbow | Effect::RainbowWheel | Effect::Off)
  }
}

fn normalize(value: &str) -> String {
  value
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .map(|c| c.to_ascii_lowercase())
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_canonical_and_alias_names() {
    assert_eq!(Effect::parse("rainbow-wheel"), Some(Effect::RainbowWheel));
    assert_eq!(Effect::parse("Rainbow Wheel"), Some(Effect::RainbowWheel));
    assert_eq!(Effect::parse("breathe"), Some(Effect::Respire));
    assert_eq!(Effect::parse("SELF_DEFINE"), Some(Effect::SelfDefine));
    assert_eq!(Effect::parse("nope"), None);
  }

  #[test]
  fn mode_numbers_match_vendor_indices() {
    assert_eq!(Effect::FixedOn.number(), 1);
    assert_eq!(Effect::Off.number(), 22);
  }
}
