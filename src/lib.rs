//! Shared device layer for the Redragon K617 RGB.
//!
//! The CLI (`src/main.rs`) and the GUI (`src/bin/redragon-k617-gui.rs`) both
//! use this crate so the wire protocol only exists in one place.

pub mod effects;
pub mod gui;
pub mod hid;
pub mod keymap;
pub mod profile;
pub mod protocol;
