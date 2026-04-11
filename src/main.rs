use anyhow::Result;
use clap::Parser;

mod cli;
mod util;
pub mod vmb;
pub mod mod_info;

use crate::cli::{Cli};

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()
}
