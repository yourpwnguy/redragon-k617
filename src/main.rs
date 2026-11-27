//! `redragon-k617` - Linux userspace driver for the Redragon K617 RGB.
//!
//! The command surface mirrors the parts of the Windows app that actually
//! touch the keyboard: firmware effects, base colour, speed, brightness,
//! profiles, plus an opt-in non-vendor per-key experiment. Everything is
//! built on the same HID feature
//! reports the vendor software uses, so no kernel driver or firmware patch is
//! involved.

use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

use redragon_k617::effects::Effect;
use redragon_k617::hid::Keyboard;
use redragon_k617::keymap;
use redragon_k617::profile::{self, Profile};
use redragon_k617::protocol::{self, EffectParams, Rgb};
use redragon_k617::snake;

#[derive(Parser, Debug)]
#[command(
  name = "redragon-k617",
  version,
  about = "Control a Redragon K617 RGB keyboard on Linux",
  long_about = "Linux userspace driver for the Redragon K617 RGB (258a:0049). \
                  It speaks the same HID feature reports as the official Windows \
                  software: 22 firmware effects, base colour, speed, brightness, \
                  JSON profiles, and an opt-in report-8 per-key experiment."
)]
struct Cli {
  /// Print the HID frames instead of sending them (no device access).
  #[arg(long, global = true)]
  dry_run: bool,

  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
  /// Show the device path, password and committed state.
  Info,
  /// List the 22 firmware effects.
  Effects,
  /// List the 61 key names.
  Keys,
  /// Read the effect currently committed by the keyboard.
  Status,
  /// Switch to a firmware effect.
  Effect {
    /// Effect name, e.g. `sine-wave`, `rainbow`, `retro-snake`.
    name: String,
    /// Base colour as RRGGBB / #RRGGBB / "r,g,b" / a name.
    #[arg(long)]
    color: Option<String>,
    /// Animation speed, 1 (slow) to 4 (fast).
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=4))]
    speed: Option<u8>,
    /// Brightness, 1 (dim) to 4 (bright).
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=4))]
    brightness: Option<u8>,
    /// Do not update the current profile.
    #[arg(long)]
    no_save: bool,
    /// Also send the direct per-key frame. This is not the vendor path and
    /// can reset some K617 firmware revisions; keep it off unless you know
    /// your keyboard accepts report 8.
    #[arg(long)]
    unsafe_per_key: bool,
  },
  /// Set the keyboard to one solid colour.
  Solid {
    /// Colour as RRGGBB / #RRGGBB / "r,g,b" / a name.
    color: String,
    /// Do not update the current profile.
    #[arg(long)]
    no_save: bool,
    /// Also send the direct per-key frame. This is not the vendor path and
    /// can reset some K617 firmware revisions; keep it off unless you know
    /// your keyboard accepts report 8.
    #[arg(long)]
    unsafe_per_key: bool,
  },
  /// Set individual keys, e.g. `paint W=ff0000 Space=00ff00`.
  ///
  /// This needs the non-vendor report-8 path, which can reset some K617
  /// firmware revisions. It is disabled unless `--unsafe-per-key` is given.
  Paint {
    /// KEY=COLOUR assignments.
    #[arg(required = true)]
    assignments: Vec<String>,
    /// Colour for every key that is not listed (default black).
    #[arg(long, default_value = "000000")]
    base: String,
    /// Do not update the current profile.
    #[arg(long)]
    no_save: bool,
    /// Also send the direct per-key frame. This is not the vendor path and
    /// can reset some K617 firmware revisions; keep it off unless you know
    /// your keyboard accepts report 8.
    #[arg(long)]
    unsafe_per_key: bool,
  },
  /// Stream the thick red S-wave snake effect.
  ///
  /// This is a host-side animation and uses the non-vendor report-8 frame.
  /// Keep the first runs short.
  Snake {
    /// Run time in seconds; 0 means until Ctrl+C.
    #[arg(long, default_value_t = 10)]
    seconds: u64,
    /// Frames per second.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=60))]
    fps: u64,
    /// Wave travel speed in radians per second.
    #[arg(long, default_value_t = 2.6)]
    speed: f32,
  },
  /// Turn the lighting off.
  Off {
    /// Do not update the current profile.
    #[arg(long)]
    no_save: bool,
  },
  /// Change brightness of the current effect.
  Brightness {
    #[arg(value_parser = clap::value_parser!(u8).range(1..=4))]
    value: u8,
  },
  /// Change animation speed of the current effect.
  Speed {
    #[arg(value_parser = clap::value_parser!(u8).range(1..=4))]
    value: u8,
  },
  /// Manage saved profiles.
  Profile {
    #[command(subcommand)]
    command: ProfileCommand,
  },
}

#[derive(Subcommand, Debug)]
enum ProfileCommand {
  /// List saved profiles.
  List,
  /// Save the current state under a name.
  Save { name: String },
  /// Apply a saved profile.
  Load {
    name: String,
    /// Also send the per-key report 8 frame. This is not the vendor path and
    /// can reset some K617 firmware revisions.
    #[arg(long)]
    unsafe_per_key: bool,
  },
  /// Delete a saved profile.
  Delete { name: String },
  /// Print the current state as JSON.
  Show,
}

fn main() {
  if let Err(error) = run(Cli::parse()) {
    eprintln!("error: {error:#}");
    std::process::exit(1);
  }
}

fn run(cli: Cli) -> Result<()> {
  match cli.command {
    Command::Info => info(cli.dry_run),
    Command::Effects => {
      for effect in Effect::ALL {
        let marker = match effect {
          Effect::Off => " (off)",
          e if !e.supports_color() => " (no base colour)",
          _ => "",
        };
        println!("{:2}  {}{}", effect.number(), effect.name(), marker);
      }
      Ok(())
    }
    Command::Keys => {
      for key in keymap::KEYS.iter() {
        println!(
          "{:<10} led={:<2} slot={}",
          key.name, key.led_index, key.sinodragon_slot
        );
      }
      Ok(())
    }
    Command::Status => status(cli.dry_run),
    Command::Effect {
      name,
      color,
      speed,
      brightness,
      no_save,
      unsafe_per_key,
    } => effect(
      cli.dry_run,
      &name,
      color,
      speed,
      brightness,
      no_save,
      unsafe_per_key,
    ),
    Command::Solid {
      color,
      no_save,
      unsafe_per_key,
    } => solid(cli.dry_run, &color, no_save, unsafe_per_key),
    Command::Paint {
      assignments,
      base,
      no_save,
      unsafe_per_key,
    } => paint(cli.dry_run, &assignments, &base, no_save, unsafe_per_key),
    Command::Snake {
      seconds,
      fps,
      speed,
    } => snake_effect(cli.dry_run, seconds, fps, speed),
    Command::Off { no_save } => {
      let profile = Profile::new(Effect::Off, Rgb::OFF, 3, 3);
      apply(cli.dry_run, &profile, false)?;
      if !no_save {
        profile::save_current(&profile)?;
      }
      Ok(())
    }
    Command::Brightness { value } => update_current(cli.dry_run, |p| p.brightness = value),
    Command::Speed { value } => update_current(cli.dry_run, |p| p.speed = value),
    Command::Profile { command } => profile_command(cli.dry_run, command),
  }
}

fn info(dry_run: bool) -> Result<()> {
  if dry_run {
    return Err(anyhow!("info needs a real device"));
  }
  let keyboard = Keyboard::open()?;
  println!(
    "device:    {} (interface {})",
    keyboard.path(),
    keyboard.interface()
  );
  let password = keyboard.password()?;
  println!(
    "password:  {}",
    password
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect::<Vec<_>>()
      .join(" ")
  );
  match keyboard.read_state() {
    Ok(state) if state.len() > protocol::STATE_MODE => {
      let mode = state[protocol::STATE_MODE];
      let name = Effect::ALL
        .into_iter()
        .find(|e| e.number() == mode)
        .map(|e| e.name())
        .unwrap_or("unknown");
      let packed = state[protocol::STATE_SPEED_BRIGHT[0]];
      println!("effect:    {mode} ({name})");
      println!("speed:     {}", (packed >> 4) & 0xf);
      println!("brightness:{}", packed & 0xf);
    }
    Ok(state) => println!("state:     short response ({} bytes)", state.len()),
    Err(error) => println!("state:     unavailable ({error})"),
  }
  Ok(())
}

fn status(dry_run: bool) -> Result<()> {
  if dry_run {
    return Err(anyhow!("status needs a real device"));
  }
  let keyboard = Keyboard::open()?;
  let state = keyboard.read_state()?;
  if state.len() <= protocol::STATE_MODE {
    return Err(anyhow!("keyboard returned a short state block"));
  }
  let mode = state[protocol::STATE_MODE];
  let name = Effect::ALL
    .into_iter()
    .find(|e| e.number() == mode)
    .map(|e| e.name())
    .unwrap_or("unknown");
  let packed = state[protocol::STATE_SPEED_BRIGHT[0]];
  println!("device:     {}", keyboard.path());
  println!("effect:     {mode} ({name})");
  println!("speed:      {}", (packed >> 4) & 0xf);
  println!("brightness: {}", packed & 0xf);
  Ok(())
}

fn effect(
  dry_run: bool,
  name: &str,
  color: Option<String>,
  speed: Option<u8>,
  brightness: Option<u8>,
  no_save: bool,
  per_key: bool,
) -> Result<()> {
  let effect = Effect::parse(name)
    .ok_or_else(|| anyhow!("unknown effect {name:?}; run `redragon-k617 effects`"))?;
  let current = profile::load_current()?;
  let color = match color {
    Some(value) => parse_color(&value)?,
    None => current
      .as_ref()
      .and_then(|p| p.color().ok())
      .unwrap_or(Rgb::new(255, 0, 0)),
  };
  let speed = speed.unwrap_or_else(|| current.as_ref().map(|p| p.speed).unwrap_or(3));
  let brightness =
    brightness.unwrap_or_else(|| current.as_ref().map(|p| p.brightness).unwrap_or(3));
  let profile = Profile::new(effect, color, speed, brightness);
  apply(dry_run, &profile, per_key)?;
  if !no_save {
    profile::save_current(&profile)?;
  }
  Ok(())
}

fn solid(dry_run: bool, color: &str, no_save: bool, per_key: bool) -> Result<()> {
  let color = parse_color(color)?;
  let current = profile::load_current()?;
  // Safe default: Fixed-on gives a real solid colour without touching the
  // fragile per-key report. Only the explicit opt-in uses Self-define plus
  // report 8.
  let effect = if per_key {
    Effect::SelfDefine
  } else {
    Effect::FixedOn
  };
  let profile = Profile::new(
    effect,
    color,
    current.as_ref().map(|p| p.speed).unwrap_or(3),
    current.as_ref().map(|p| p.brightness).unwrap_or(3),
  );
  apply(dry_run, &profile, per_key)?;
  if !no_save {
    profile::save_current(&profile)?;
  }
  Ok(())
}

fn paint(
  dry_run: bool,
  assignments: &[String],
  base: &str,
  no_save: bool,
  per_key: bool,
) -> Result<()> {
  if !per_key {
    return Err(anyhow!(
      "per-key painting needs the non-vendor report 8 path, which can reset some K617 firmware revisions; re-run with --unsafe-per-key only if you accept that risk."
    ));
  }
  let base = parse_color(base)?;
  let current = profile::load_current()?;
  let mut profile = Profile::new(
    Effect::SelfDefine,
    base,
    current.as_ref().map(|p| p.speed).unwrap_or(3),
    current.as_ref().map(|p| p.brightness).unwrap_or(3),
  );
  for assignment in assignments {
    let (key_name, color_name) = assignment
      .split_once('=')
      .ok_or_else(|| anyhow!("expected KEY=COLOUR, got {assignment:?}"))?;
    let key = keymap::find(key_name)
      .ok_or_else(|| anyhow!("unknown key {key_name:?}; run `redragon-k617 keys`"))?;
    let color = parse_color(color_name)?;
    profile.keys.insert(key.name.to_string(), color.to_hex());
  }
  apply(dry_run, &profile, per_key)?;
  if !no_save {
    profile::save_current(&profile)?;
  }
  Ok(())
}

/// Stream the thick red S-wave snake. This is the non-vendor report-8 path:
/// it works on the tested K617 with red but can be fragile on other firmware
/// revisions, so keep the first runs short and stop if the device resets.
fn snake_effect(dry_run: bool, seconds: u64, fps: u64, speed: f32) -> Result<()> {
  if dry_run {
    for phase in [
      0.0,
      std::f32::consts::FRAC_PI_2,
      std::f32::consts::PI,
      3.0 * std::f32::consts::FRAC_PI_2,
    ] {
      let lit: Vec<&str> = snake::frame(phase)
        .into_iter()
        .filter(|(_, color)| *color != Rgb::OFF)
        .map(|(key, _)| key.name)
        .collect();
      println!("phase {phase:.2}: {}", lit.join(" "));
    }
    return Ok(());
  }

  let fps = fps.clamp(1, 60);
  let speed = speed.clamp(0.2, 8.0);
  let keyboard = Keyboard::open()?;
  let previous = profile::load_current()?;

  // Self-define is where report 8 lands. Red is the vendor default base
  // colour and the colour used by the mockup.
  keyboard.apply_effect(EffectParams {
    effect: Effect::SelfDefine,
    color: Rgb::new(255, 0, 0),
    speed: 3,
    brightness: 4,
  })?;

  eprintln!(
    "snake: streaming report 8 at {fps} fps; {}",
    if seconds == 0 {
      "press Ctrl+C to stop".to_string()
    } else {
      format!("running for {seconds}s")
    }
  );

  let frame_time = Duration::from_secs_f64(1.0 / fps as f64);
  let start = Instant::now();
  let limit = (seconds > 0).then(|| Duration::from_secs(seconds));
  let mut phase = 0.0f32;

  loop {
    let frame = protocol::per_key_frame(&snake::frame(phase));
    keyboard.send_per_key(&frame)?;
    if let Some(limit) = limit {
      if start.elapsed() >= limit {
        break;
      }
    }
    std::thread::sleep(frame_time);
    phase = (phase + speed * frame_time.as_secs_f32()) % std::f32::consts::TAU;
  }

  drop(keyboard);
  match previous {
    Some(profile) => {
      apply(false, &profile, false)?;
      eprintln!("snake: restored {}", profile.effect);
    }
    None => {
      let fallback = Profile::new(Effect::Rainbow, Rgb::new(0, 255, 0), 3, 3);
      apply(false, &fallback, false)?;
      eprintln!("snake: restored rainbow");
    }
  }
  Ok(())
}

fn update_current(dry_run: bool, mutate: impl FnOnce(&mut Profile)) -> Result<()> {
  let mut profile = profile::load_current()
    .context("reading the current profile")?
    .ok_or_else(|| anyhow!("no current profile yet; set an effect or colour first"))?;
  mutate(&mut profile);
  apply(dry_run, &profile, false)?;
  if !dry_run {
    profile::save_current(&profile)?;
  }
  Ok(())
}

fn profile_command(dry_run: bool, command: ProfileCommand) -> Result<()> {
  match command {
    ProfileCommand::List => {
      let names = profile::list_named()?;
      if names.is_empty() {
        println!("no saved profiles");
      }
      for name in names {
        println!("{name}");
      }
      Ok(())
    }
    ProfileCommand::Save { name } => {
      let current = profile::load_current()?
        .ok_or_else(|| anyhow!("no current state to save; set an effect first"))?;
      let path = profile::save_named(&name, &current)?;
      println!("saved {} to {}", name, path.display());
      Ok(())
    }
    ProfileCommand::Load {
      name,
      unsafe_per_key,
    } => {
      let profile = profile::load_named(&name)?;
      if !unsafe_per_key && !profile.keys.is_empty() {
        eprintln!(
          "warning: {name} contains per-key colours; they are preview-only because report 8 can reset the keyboard. Pass --unsafe-per-key to send them."
        );
      }
      apply(dry_run, &profile, unsafe_per_key)?;
      if !dry_run {
        profile::save_current(&profile)?;
      }
      println!("loaded {name}");
      Ok(())
    }
    ProfileCommand::Delete { name } => {
      profile::delete_named(&name)?;
      println!("deleted {name}");
      Ok(())
    }
    ProfileCommand::Show => {
      let profile = profile::load_current()?.ok_or_else(|| anyhow!("no current profile yet"))?;
      println!("{}", serde_json::to_string_pretty(&profile)?);
      Ok(())
    }
  }
}

/// Apply a profile. The five-frame firmware burst is always sent. The
/// optional report-8 per-key frame is non-vendor: the official app uses
/// report 6 LED-matrix blocks instead, and report 8 has been observed to
/// reset some K617 firmware revisions when the base colour changes.
fn apply(dry_run: bool, profile: &Profile, per_key: bool) -> Result<()> {
  let effect = profile.effect()?;
  let color = profile.color()?;
  let params = EffectParams {
    effect,
    color,
    speed: profile.speed,
    brightness: profile.brightness,
  };
  if effect == Effect::SelfDefine {
    if !per_key {
      if dry_run {
        for (index, frame) in protocol::effect_burst(params).iter().enumerate() {
          print_frame(&format!("frame {index}"), frame);
        }
        return Ok(());
      }
      let keyboard = Keyboard::open()?;
      keyboard.apply_effect(params)?;
      return Ok(());
    }
    let overrides = profile.key_overrides()?;
    let frame = if overrides.is_empty() {
      protocol::solid_frame(color)
    } else {
      protocol::per_key_frame_with_base(color, &overrides)
    };
    if dry_run {
      print_frame("per-key", &frame);
      print_frame("commit", &protocol::effect_burst(params)[4]);
      return Ok(());
    }
    let keyboard = Keyboard::open()?;
    keyboard.apply_effect(params)?;
    keyboard.send_per_key(&frame)?;
  } else {
    if dry_run {
      for (index, frame) in protocol::effect_burst(params).iter().enumerate() {
        print_frame(&format!("frame {index}"), frame);
      }
      return Ok(());
    }
    let keyboard = Keyboard::open()?;
    keyboard.apply_effect(params)?;
  }
  Ok(())
}

fn print_frame(label: &str, frame: &[u8]) {
  let head = &frame[..frame.len().min(16)];
  println!(
    "{label}: {} bytes  {}",
    frame.len(),
    head
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect::<Vec<_>>()
      .join(" ")
  );
}

fn parse_color(value: &str) -> Result<Rgb> {
  Rgb::parse(value)
    .ok_or_else(|| anyhow!("invalid colour {value:?}; use RRGGBB, #RRGGBB, r,g,b or a colour name"))
}
