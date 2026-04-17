use anyhow::Result;
use clap::Parser;

mod cli;
mod game_wrapper;
mod rendering_api;
mod util;
pub mod vmb;
pub mod mod_info;

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
