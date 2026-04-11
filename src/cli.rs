use std::path::PathBuf;

use crate::vmb::Vmb;
use clap::builder::styling;
use clap::{Parser, Subcommand};

const CLI_STYLE: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::White.on_default().underline().bold())
    .usage(styling::AnsiColor::White.on_default().bold())
    .error(styling::AnsiColor::BrightRed.on_default().bold())
    .valid(styling::AnsiColor::White.on_default().bold())
    .invalid(styling::AnsiColor::White.on_default())
    .literal(styling::AnsiColor::White.on_default().bold())
    .placeholder(styling::AnsiColor::BrightBlack.on_default());

const CLI_HELP_TEMPLATE: &str = "\
{about-with-newline}\
by {author-with-newline}\n\
Usage:\n    {usage}\n\n\
{all-args}\n\
{after-help}";

#[derive(Parser, Debug)]
#[command(
    author = clap::crate_authors!(),
    version = clap::crate_version!(),
    about = clap::crate_description!(),
    help_template = CLI_HELP_TEMPLATE,
    styles = CLI_STYLE
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}


#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Initialize the given path with mod boilerplate
    Init {
        /// Root path to initialize
        mod_path: PathBuf,
        /// Skip git repository initialization
        #[arg(long = "no-git")]
        no_git: bool,
        /// Modworkshop id for providing updates
        #[arg(long = "update-id")]
        update_id: Option<u32>,
    },
    /// Package one or more files/directories into a .vmz archive
    Pack {
        /// Output archive path (.vmz extension is enforced)
        #[arg(short, long)]
        output: PathBuf,
        /// Files/directories to include
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
    },
    /// Install a [.zip|.vmz] archive or a mod root directory into an auto-detected or provided directory
    Install {
        /// Archive path or mod root directory to install
        #[arg(default_value = ".")]
        source: PathBuf,
        /// Install directory (used when auto-detection is unavailable)
        path: Option<PathBuf>,
    },
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Init { mod_path, no_git, update_id } => Vmb::init(mod_path, no_git, update_id),
            Command::Pack { output, inputs } => Vmb::pack(output, inputs),
            Command::Install { source, path } => Vmb::install(source, path),
        }
    }
}
