//! Physical key map for the K617 (61 keys, US ANSI).
//!
//! `led_index` is the index used by the vendor profile format, in reading
//! order (left to right, top to bottom). `sinodragon_slot` is the position in
//! the 96-slot, column-major LED matrix used by report 8. The anchors
//! (Escape=0, Tab=2, CapsLock=3, LShift=4, LCtrl=5, J=45, Space=35, Enter=81)
//! were verified on hardware; the mapping is the same one the vendor software
//! uses for the per-key editor.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyDef {
  pub name: &'static str,
  pub led_index: usize,
  pub sinodragon_slot: u8,
}

macro_rules! key {
  ($name:literal, $led:expr, $slot:expr) => {
    KeyDef {
      name: $name,
      led_index: $led,
      sinodragon_slot: $slot,
    }
  };
}

pub const KEYS: [KeyDef; KEY_COUNT] = [
  // Row 0: number row
  key!("Escape", 0, 0),
  key!("1", 1, 7),
  key!("2", 2, 13),
  key!("3", 3, 19),
  key!("4", 4, 25),
  key!("5", 5, 31),
  key!("6", 6, 37),
  key!("7", 7, 43),
  key!("8", 8, 49),
  key!("9", 9, 55),
  key!("0", 10, 61),
  key!("Minus", 11, 67),
  key!("Equal", 12, 73),
  key!("Backspace", 13, 79),
  // Row 1: QWERTY
  key!("Tab", 14, 2),
  key!("Q", 15, 8),
  key!("W", 16, 14),
  key!("E", 17, 20),
  key!("R", 18, 26),
  key!("T", 19, 32),
  key!("Y", 20, 38),
  key!("U", 21, 44),
  key!("I", 22, 50),
  key!("O", 23, 56),
  key!("P", 24, 62),
  key!("LBracket", 25, 68),
  key!("RBracket", 26, 74),
  key!("Backslash", 27, 80),
  // Row 2: ASDF
  key!("CapsLock", 28, 3),
  key!("A", 29, 9),
  key!("S", 30, 15),
  key!("D", 31, 21),
  key!("F", 32, 27),
  key!("G", 33, 33),
  key!("H", 34, 39),
  key!("J", 35, 45),
  key!("K", 36, 51),
  key!("L", 37, 57),
  key!("Semicolon", 38, 63),
  key!("Quote", 39, 69),
  key!("Enter", 40, 81),
  // Row 3: ZXCV
  key!("LShift", 41, 4),
  key!("Z", 42, 10),
  key!("X", 43, 16),
  key!("C", 44, 22),
  key!("V", 45, 28),
  key!("B", 46, 34),
  key!("N", 47, 40),
  key!("M", 48, 46),
  key!("Comma", 49, 52),
  key!("Period", 50, 58),
  key!("Slash", 51, 64),
  key!("RShift", 52, 82),
  // Row 4: bottom row
  key!("LCtrl", 53, 5),
  key!("LSuper", 54, 11),
  key!("LAlt", 55, 17),
  key!("Space", 56, 35),
  key!("RAlt", 57, 53),
  key!("Fn", 58, 59),
  key!("Menu", 59, 77),
  key!("RCtrl", 60, 83),
];

pub const KEY_COUNT: usize = 61;

/// Key centres in the 560x212 preview image, in the same order as `KEYS`.
/// Used by host-side previews such as the snake animation.
pub const CENTERS: [(f32, f32); KEY_COUNT] = [
  (25.0, 28.0),   // Escape
  (61.0, 28.0),   // 1
  (97.0, 28.0),   // 2
  (134.0, 28.0),  // 3
  (170.0, 28.0),  // 4
  (207.0, 28.0),  // 5
  (244.0, 29.0),  // 6
  (280.0, 28.0),  // 7
  (316.0, 28.0),  // 8
  (353.0, 28.0),  // 9
  (390.0, 28.0),  // 0
  (426.0, 28.0),  // Minus
  (462.0, 28.0),  // Equal
  (516.5, 28.0),  // Backspace
  (33.5, 65.0),   // Tab
  (79.0, 65.0),   // Q
  (116.0, 65.0),  // W
  (152.0, 65.0),  // E
  (189.0, 64.0),  // R
  (225.0, 64.0),  // T
  (262.0, 64.0),  // Y
  (298.0, 64.0),  // U
  (334.0, 64.0),  // I
  (371.0, 65.0),  // O
  (407.0, 65.0),  // P
  (444.0, 65.0),  // LBracket
  (480.0, 65.0),  // RBracket
  (526.0, 65.0),  // Backslash
  (39.5, 100.0),  // CapsLock
  (89.0, 101.0),  // A
  (125.0, 101.0), // S
  (161.0, 101.0), // D
  (198.0, 101.0), // F
  (235.0, 101.0), // G
  (271.0, 101.0), // H
  (308.0, 101.0), // J
  (344.0, 101.0), // K
  (380.0, 101.0), // L
  (417.0, 101.0), // Semicolon
  (453.0, 101.0), // Quote
  (512.0, 101.0), // Enter
  (48.5, 137.0),  // LShift
  (106.0, 137.0), // Z
  (144.0, 137.0), // X
  (180.0, 137.0), // C
  (216.0, 137.0), // V
  (252.0, 137.0), // B
  (289.0, 137.0), // N
  (325.0, 137.0), // M
  (361.0, 137.0), // Comma
  (398.0, 137.0), // Period
  (434.0, 137.0), // Slash
  (502.5, 137.0), // RShift
  (28.0, 174.0),  // LCtrl
  (75.0, 174.0),  // LSuper
  (121.0, 174.0), // LAlt
  (257.0, 174.0), // Space
  (393.0, 174.0), // RAlt
  (439.5, 174.0), // Fn
  (484.5, 174.0), // Menu
  (530.5, 174.0), // RCtrl
];

/// Resolve a user-supplied key name. Besides the canonical names above this
/// accepts the common short forms (`Esc`, `Ctrl`, `Win`, punctuation names).
pub fn find(name: &str) -> Option<&'static KeyDef> {
  let normalized = normalize(name);
  KEYS.iter().find(|key| {
    normalize(key.name) == normalized
      || aliases(key.name)
        .iter()
        .any(|alias| normalize(alias) == normalized)
  })
}

fn normalize(value: &str) -> String {
  value
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .map(|c| c.to_ascii_lowercase())
    .collect()
}

fn aliases(canonical: &str) -> &'static [&'static str] {
  match canonical {
    "Escape" => &["Esc"],
    "Backspace" => &["Bksp", "Back"],
    "CapsLock" => &["Caps", "Caps Lock"],
    "LBracket" => &["[", "LeftBracket", "OpenBracket"],
    "RBracket" => &["]", "RightBracket", "CloseBracket"],
    "Backslash" => &["\\", "Pipe"],
    "Semicolon" => &[";"],
    "Quote" => &["'", "Apostrophe"],
    "Enter" => &["Return"],
    "LShift" => &["LeftShift", "Shift"],
    "RShift" => &["RightShift"],
    "LCtrl" => &["LeftCtrl", "Ctrl", "Control"],
    "RCtrl" => &["RightCtrl"],
    "LSuper" => &["LeftSuper", "Win", "Windows", "Super", "Meta"],
    "LAlt" => &["LeftAlt", "Alt"],
    "RAlt" => &["RightAlt", "AltGr"],
    "Comma" => &[",", "Comma"],
    "Period" => &[".", "Dot", "FullStop"],
    "Slash" => &["/", "ForwardSlash"],
    "Minus" => &["-", "Dash"],
    "Equal" => &["=", "Equals", "Plus"],
    "Space" => &["Spacebar", " "],
    _ => &[],
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn led_indices_are_unique_and_contiguous() {
    let mut seen = [false; KEY_COUNT];
    for key in KEYS.iter() {
      assert!(
        !seen[key.led_index],
        "duplicate led index {}",
        key.led_index
      );
      seen[key.led_index] = true;
    }
    assert!(seen.iter().all(|v| *v));
  }

  #[test]
  fn sinodragon_slots_are_unique_and_in_range() {
    let mut seen = [false; 96];
    for key in KEYS.iter() {
      let slot = key.sinodragon_slot as usize;
      assert!(slot < 96, "{} has out-of-range slot {}", key.name, slot);
      assert!(!seen[slot], "{} reuses slot {}", key.name, slot);
      seen[slot] = true;
    }
  }

  #[test]
  fn resolves_aliases() {
    assert_eq!(find("esc").unwrap().led_index, 0);
    assert_eq!(find("space").unwrap().sinodragon_slot, 35);
    assert_eq!(find("Return").unwrap().led_index, 40);
    assert_eq!(find("ctrl").unwrap().led_index, 53);
    assert!(find("F13").is_none());
  }
}
