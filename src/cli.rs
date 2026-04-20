use crate::mod_info::{ModInfo, ModInfoOverride, ModUpdatesInfo};
use crate::rendering_api::RenderingAPI;
use crate::util::derive_dir_name;
use crate::vmb::Vmb;
use anyhow::bail;
use clap::builder::styling;
use clap::{Parser, Subcommand};
use semver::Version;
use std::path::PathBuf;

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
		/// Name for the mod
		#[arg(short, long)]
		name: Option<String>,
		/// ID for the mod
		#[arg(short, long)]
		id: Option<String>,
		/// Priority for the mod used in the mod loader
		#[arg(short, long)]
		priority: Option<i32>,
		/// Semver formatted version
		#[arg(short = 'v')]
		version: Option<Version>,
		/// Modworkshop id for providing updates
		#[arg(short = 'u', long = "up-id")]
		update_id: Option<u32>,
		/// Skip git repository initialization
		#[arg(long = "no-git")]
		no_git: bool,
	},
	/// Modify parameters of the mod
	Modify {
		/// Root path with mod.txt
		#[arg(default_value = ".")]
		mod_path: PathBuf,
		/// Name for the mod
		#[arg(short, long)]
		name: Option<String>,
		/// ID for the mod
		#[arg(short, long)]
		id: Option<String>,
		/// Priority for the mod used in the mod loader
		#[arg(short, long)]
		priority: Option<i32>,
		/// Semver formatted version
		#[arg(short = 'v')]
		version: Option<Version>,
		/// Modworkshop id for providing updates
		#[arg(short = 'u', long = "up-id")]
		update_id: Option<u32>,
	},
	/// Package one or more files/directories into a .vmz archive
	Pack {
		/// Output archive path (.vmz extension is enforced)
		#[arg(short, long)]
		output: PathBuf,
		/// Files/directories to include [defaults to ./mod.txt and ./mods when omitted]
		inputs: Vec<PathBuf>,
	},
	/// Install a [.zip|.vmz] archive or a mod root directory into an auto-detected or provided directory
	Install {
		/// Archive path or mod root directory to install
		#[arg(default_value = ".")]
		source: PathBuf,
		/// Install directory (used when auto-detection is unavailable and VOSTOK_PATH is not provided)
		path: Option<PathBuf>,
	},
	/// Displays the latest output log (if available)
	Log {
		/// Watches for changes to the output log
		#[arg(short, long)]
		watch: bool,
	},
	/// Runs the game and streams the log output
	Run {
		/// Rendering API
		#[arg(short, long, value_enum)]
		api: Option<RenderingAPI>,
		/// Additional arguments passed to the game executable
		#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
		args: Vec<String>,
	},
	/// Self-management commands
	#[command(name = "self")]
	SelfCmd {
		#[command(subcommand)]
		command: SelfCommand,
	},
}

#[derive(Subcommand, Debug)]
pub(crate) enum SelfCommand {
	/// Updates the app to the latest release
	Update,
}

impl Cli {
	pub fn run(self) -> anyhow::Result<()> {
		match self.command {
			Command::Init {
				mod_path,
				name,
				id,
				priority,
				version,
				update_id,
				no_git,
			} => {
				let mod_name = derive_dir_name(&mod_path)?;
				let mut mod_info = ModInfo::default_from(mod_name);

				let mut mod_info_override = ModInfoOverride {
					name,
					id,
					priority,
					version,
					autoload: None,
					updates: None,
				};
				if let Some(update_id) = update_id {
					mod_info_override.updates = Some(ModUpdatesInfo {
						modworkshop: update_id,
					});
				}
				Vmb::modify(mod_path.as_path(), &mut mod_info, mod_info_override, true, false)?;
				Vmb::init(mod_path, no_git, mod_info)
			}
			Command::Modify {
				mod_path,
				name,
				id,
				priority,
				version,
				update_id,
			} => {
				let info_path = mod_path.join("mod.txt");
				if !info_path.is_file() {
					bail!("info file does not exist");
				}
				let mut mod_info = ModInfo::from_path(&info_path)?;

				let mut mod_info_override = ModInfoOverride {
					name,
					id,
					priority,
					version,
					autoload: None,
					updates: None,
				};
				if let Some(update_id) = update_id {
					mod_info_override.updates = Some(ModUpdatesInfo {
						modworkshop: update_id,
					});
				}
				Vmb::modify(mod_path.as_path(), &mut mod_info, mod_info_override, false, true)
			}
			Command::Pack { output, inputs } => Vmb::pack(output, inputs),
			Command::Install { source, path } => Vmb::install(source, path),
			Command::Log { watch } => Vmb::log(watch),
			Command::Run {
				api,
				args,
			} => Vmb::run(None, api, args),
			Command::SelfCmd { command } => match command {
				SelfCommand::Update => Vmb::update(),
			},
		}
	}
}
