//! On-disk profiles.
//!
//! The vendor app stores profiles in proprietary `.pro` files. We cannot read
//! those without reimplementing the whole document model, so the Linux port
//! uses a small JSON schema that captures the same user-facing state: effect,
//! base colour, speed, brightness and per-key overrides.
//!
//! Profiles live under `$XDG_CONFIG_HOME/redragon-k617` (or
//! `~/.config/redragon-k617`). `current.json` is the state that
//! `brightness`/`speed` modify.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::effects::Effect;
use crate::protocol::Rgb;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
  /// Canonical effect name, e.g. `sine-wave`.
  pub effect: String,
  /// Base colour as `rrggbb`.
  pub color: String,
  /// 1..=4.
  pub speed: u8,
  /// 1..=4.
  pub brightness: u8,
  /// Per-key overrides keyed by canonical key name (`W`, `Space`, ...).
  #[serde(default)]
  pub keys: BTreeMap<String, String>,
}

impl Profile {
  pub fn new(effect: Effect, color: Rgb, speed: u8, brightness: u8) -> Self {
    Self {
      effect: effect.name().to_string(),
      color: color.to_hex(),
      speed,
      brightness,
      keys: BTreeMap::new(),
    }
  }

  pub fn effect(&self) -> Result<Effect> {
    Effect::parse(&self.effect)
      .ok_or_else(|| anyhow!("profile contains unknown effect {:?}", self.effect))
  }

  pub fn color(&self) -> Result<Rgb> {
    Rgb::parse(&self.color)
      .ok_or_else(|| anyhow!("profile contains invalid colour {:?}", self.color))
  }

  pub fn key_overrides(&self) -> Result<Vec<(usize, Rgb)>> {
    let mut parsed = Vec::new();
    for (name, color) in &self.keys {
      let key = crate::keymap::find(name)
        .ok_or_else(|| anyhow!("profile contains unknown key {name:?}"))?;
      let color = Rgb::parse(color)
        .ok_or_else(|| anyhow!("profile contains invalid colour {color:?} for {name}"))?;
      parsed.push((key.led_index, color));
    }
    Ok(parsed)
  }
}

pub fn config_dir() -> Result<PathBuf> {
  if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
    if !xdg.is_empty() {
      return Ok(PathBuf::from(xdg).join("redragon-k617"));
    }
  }
  let home = std::env::var_os("HOME")
    .ok_or_else(|| anyhow!("HOME is not set; cannot locate the config directory"))?;
  Ok(PathBuf::from(home).join(".config").join("redragon-k617"))
}

pub fn profiles_dir() -> Result<PathBuf> {
  Ok(config_dir()?.join("profiles"))
}

pub fn current_path() -> Result<PathBuf> {
  Ok(config_dir()?.join("current.json"))
}

pub fn save_current(profile: &Profile) -> Result<()> {
  write_json(&current_path()?, profile)
}

pub fn load_current() -> Result<Option<Profile>> {
  let path = current_path()?;
  if !path.exists() {
    return Ok(None);
  }
  Ok(Some(read_json(&path)?))
}

pub fn save_named(name: &str, profile: &Profile) -> Result<PathBuf> {
  let path = profile_path(name)?;
  write_json(&path, profile)?;
  Ok(path)
}

pub fn load_named(name: &str) -> Result<Profile> {
  read_json(&profile_path(name)?)
}

pub fn delete_named(name: &str) -> Result<()> {
  let path = profile_path(name)?;
  fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))
}

pub fn list_named() -> Result<Vec<String>> {
  let dir = profiles_dir()?;
  if !dir.exists() {
    return Ok(Vec::new());
  }
  let mut names = Vec::new();
  for entry in fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
    let entry = entry?;
    let path = entry.path();
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
      if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        names.push(stem.to_string());
      }
    }
  }
  names.sort();
  Ok(names)
}

fn profile_path(name: &str) -> Result<PathBuf> {
  let safe = name
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
  if !safe || name.is_empty() {
    return Err(anyhow!(
      "invalid profile name {name:?}; use letters, digits, '-' or '_'"
    ));
  }
  Ok(profiles_dir()?.join(format!("{name}.json")))
}

fn read_json(path: &Path) -> Result<Profile> {
  let data = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
  serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))
}

fn write_json(path: &Path, profile: &Profile) -> Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
  }
  let data = serde_json::to_string_pretty(profile)?;
  fs::write(path, data + "\n").with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn profile_round_trips_through_json() {
    let mut profile = Profile::new(Effect::SineWave, Rgb::new(1, 2, 3), 3, 4);
    profile.keys.insert("W".into(), "ff0000".into());
    let json = serde_json::to_string(&profile).unwrap();
    let parsed: Profile = serde_json::from_str(&json).unwrap();
    assert_eq!(profile, parsed);
    assert_eq!(
      parsed.key_overrides().unwrap(),
      vec![(16, Rgb::new(255, 0, 0))]
    );
  }
}
