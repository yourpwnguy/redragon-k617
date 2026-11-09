//! Development note: run the CLI tests after touching the GUI; the protocol
//! crate is shared, so a change in `protocol.rs` affects both front ends.

fn main() -> eframe::Result<()> {
  redragon_k617::gui::run()
}
