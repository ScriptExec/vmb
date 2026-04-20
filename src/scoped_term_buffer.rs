use std::io::{IsTerminal, Write};
use anyhow::Context;

pub struct ScopedTermBuffer {
	enabled: bool,
}

impl ScopedTermBuffer {
	pub fn enter() -> anyhow::Result<Self> {
		if !std::io::stdout().is_terminal() {
			return Ok(Self { enabled: false });
		}

		let mut stdout = std::io::stdout();
		stdout
			.write_all(b"\x1b[?1049h\x1b[2J\x1b[H")
			.context("failed to switch to alternate screen buffer")?;
		stdout
			.flush()
			.context("failed to flush stdout after entering alternate screen")?;

		Ok(Self { enabled: true })
	}
}

impl Drop for ScopedTermBuffer {
	fn drop(&mut self) {
		if !self.enabled {
			return;
		}

		let mut stdout = std::io::stdout();
		let _ = stdout.write_all(b"\x1b[?1049l");
		let _ = stdout.flush();
	}
}