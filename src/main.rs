use anyhow::Result;
use clap::Parser;

mod cli;
mod game_wrapper;
mod progress;
mod rendering_api;
mod util;
pub mod vmb;
pub mod mod_info;
pub mod scoped_term_buffer;
pub mod mod_package;

use crate::cli::Cli;
use crate::util::print_error;

fn main() -> Result<()> {
	let cli = Cli::parse();
	if let Err(err) = cli.run() {
		print_error(format!("{err:#}"));
		std::process::exit(1);
	}

	Ok(())
}
