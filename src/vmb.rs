use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{IsTerminal, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, SystemTime};

use crate::game_wrapper::GameWrapper;
use crate::mod_info::{ModInfo, ModInfoOverride};
use crate::rendering_api::RenderingAPI;
use crate::util::{derive_dir_name, print_status, to_safe_name, to_skewer_case};
use anyhow::{bail, Context, Result};
use directories::BaseDirs;
use git2::Repository;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use tempfile::TempDir;
use time::OffsetDateTime;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::DateTime;
use crate::scoped_term_buffer::ScopedTermBuffer;

include!(concat!(env!("OUT_DIR"), "/generated_templates.rs"));

pub struct Vmb;

impl Vmb {
	const VOSTOK_APP_ID: i32 = 1963610;
	const VOSTOK_APP_NAME: &'static str = "Road to Vostok";
	const WINDOWS_DEFAULT_EXE_PATH: &'static str =
		"C:\\Program Files (x86)\\Steam\\steamapps\\common\\Road to Vostok\\";
	const LINUX_DEFAULT_EXE_PATH: &'static str =
		"~/.steam/steam/steamapps/common/Road to Vostok/";
	const LOG_NAME: &str = "godot.log";

	fn load_template(name: &str) -> Result<&str> {
		let templates = template_map();
		templates
			.get(name)
			.with_context(|| format!("Template not found: {}", name))
			.copied()
	}

	pub fn init(mod_path: PathBuf, no_git: bool, mod_info: ModInfo) -> Result<()> {
		let mod_name = derive_dir_name(&mod_path)?;
		let mod_safe_name = to_safe_name(&mod_name);

		let main_gd_template = Self::load_template("Main.gd")?;
		let gitignore_template = if no_git {
			None
		} else {
			Some(Self::load_template(".gitignore")?)
		};

		fs::create_dir_all(&mod_path).with_context(|| {
			format!(
				"failed to create or access mod directory: {}",
				mod_path.display()
			)
		})?;

		let mod_info_path = mod_path.join("mod.txt");
		let main_gd_path = mod_path.join("mods").join(&mod_safe_name).join("Main.gd");
		let gitignore_path = mod_path.join(".gitignore");

		if mod_info_path.exists() {
			bail!(
                "refusing to overwrite existing file: {}",
                mod_info_path.display()
            );
		}
		if main_gd_path.exists() {
			bail!(
                "refusing to overwrite existing file: {}",
                main_gd_path.display()
            );
		}

		if let Some(parent) = main_gd_path.parent() {
			fs::create_dir_all(parent)
				.with_context(|| format!("failed to create directory: {}", parent.display()))?;
		}

		mod_info
			.write(&mod_info_path)
			.with_context(|| format!("failed to write mod info to {}", mod_info_path.display()))?;

		fs::write(
			&main_gd_path,
			Self::replace_macros(main_gd_template, &mod_name),
		)
			.with_context(|| format!("failed to write {}", main_gd_path.display()))?;

		if no_git {
			print_status(
				"Skipping",
				"git repository initialization (\"--no-git\" specified)",
			);
		} else {
			if !gitignore_path.exists() {
				fs::write(&gitignore_path, gitignore_template.unwrap())
					.with_context(|| format!("failed to write {}", gitignore_path.display()))?;
			} else {
				print_status(
					"Skipping",
					format!("{} already exists", gitignore_path.display()),
				);
			}

			Self::init_git_repo(&mod_path)?;
		}

		print_status(
			"Initialized",
			format!("mod '{}' at {}", mod_name, mod_path.display()),
		);
		Ok(())
	}

	fn init_git_repo(mod_path: &Path) -> Result<()> {
		let git_dir = mod_path.join(".git");

		if git_dir.is_dir() {
			return Ok(());
		}
		if git_dir.exists() {
			bail!(".git exists but is not a directory: {}", git_dir.display());
		}

		Repository::init(mod_path).with_context(|| {
			format!(
				"failed to initialize git repository in {}",
				mod_path.display()
			)
		})?;
		Ok(())
	}

	pub fn pack(output: PathBuf, inputs: Vec<PathBuf>) -> Result<()> {
		let output = Self::enforce_extension(output, "vmz");

		if inputs.is_empty() {
			bail!("at least one input file or directory is required");
		}

		if let Some(parent) = output.parent() {
			if !parent.as_os_str().is_empty() {
				fs::create_dir_all(parent).with_context(|| {
					format!("failed to create output directory: {}", parent.display())
				})?;
			}
		}

		let out_file = File::create(&output)
			.with_context(|| format!("failed to create archive: {}", output.display()))?;
		let mut writer = zip::ZipWriter::new(out_file);
		let mut used_names = HashSet::new();

		for input in inputs {
			if !input.exists() {
				bail!("input path does not exist: {}", input.display());
			}

			if input.is_file() {
				let file_name = input
					.file_name()
					.and_then(|n| n.to_str())
					.map(str::to_owned)
					.with_context(|| format!("invalid file name: {}", input.display()))?;

				Self::add_file_to_zip(
					&mut writer,
					&input,
					&file_name,
					CompressionMethod::Deflated,
					&mut used_names,
				)?;
				continue;
			}

			let root_name = input
				.file_name()
				.and_then(|n| n.to_str())
				.map(str::to_owned)
				.unwrap_or_else(|| "root".to_string());

			for entry in WalkDir::new(&input).into_iter().filter_map(|e| e.ok()) {
				let path = entry.path();
				if path.is_dir() {
					continue;
				}

				let rel = path.strip_prefix(&input).with_context(|| {
					format!("failed to compute relative path for {}", path.display())
				})?;
				let archive_name = Self::path_to_zip_name(Path::new(&root_name).join(rel));
				Self::add_file_to_zip(
					&mut writer,
					path,
					&archive_name,
					CompressionMethod::Deflated,
					&mut used_names,
				)?;
			}
		}

		writer.finish().context("failed to finalize archive")?;
		print_status("Created", output.display().to_string());
		Ok(())
	}

	pub fn install(source: PathBuf, path: Option<PathBuf>) -> Result<()> {
		let (archive, _temp_dir) = Self::prepare_install_source(source)?;

		let target_dir = Self::resolve_install_dir(path)?;
		fs::create_dir_all(&target_dir).with_context(|| {
			format!(
				"failed to create install directory: {}",
				target_dir.display()
			)
		})?;

		Self::install_archive(&archive, &target_dir)
	}

	fn prepare_install_source(source: PathBuf) -> Result<(PathBuf, Option<TempDir>)> {
		if source.is_file() {
			let ext = source
				.extension()
				.and_then(|e| e.to_str())
				.map(|e| e.to_ascii_lowercase())
				.unwrap_or_default();
			if ext != "zip" && ext != "vmz" {
				bail!("archive must be .zip or .vmz: {}", source.display());
			}
			return Ok((source, None));
		}

		if !source.is_dir() {
			bail!(
                "source does not exist or is not a file/directory: {}",
                source.display()
            );
		}

		if !Self::is_mod_root(&source) {
			bail!(
                "directory is not a mod root (expected mod.txt and mods/): {}",
                source.display()
            );
		}

		let mod_name = derive_dir_name(&source)?;
		let temp_dir = TempDir::new().context("failed to create temporary directory")?;
		let temp_archive = temp_dir
			.path()
			.join(format!("{}.vmz", to_safe_name(&mod_name)));
		let inputs = vec![source.join("mod.txt"), source.join("mods")];
		Self::pack(temp_archive.clone(), inputs)
			.with_context(|| format!("failed to package mod root {}", source.display()))?;

		Ok((temp_archive, Some(temp_dir)))
	}

	fn install_archive(archive: &Path, target_dir: &Path) -> Result<()> {

		let file_name = archive
			.file_name()
			.context("archive path has no file name")?;

		let mut destination = target_dir.join(file_name);
		if let Some(ext) = archive.extension() && ext == "zip" {
			destination.set_extension("vmz");
		}

		fs::copy(&archive, &destination).with_context(|| {
			format!(
				"failed to copy {} to {}",
				archive.display(),
				destination.display()
			)
		})?;

		print_status("Installed", destination.display().to_string());
		Ok(())
	}

	/// Expects ./mod.txt and ./mods dir to exist, but does not validate their contents
	fn is_mod_root(path: &Path) -> bool {
		path.join("mod.txt").is_file() && path.join("mods").is_dir()
	}

	fn resolve_install_dir(path: Option<PathBuf>) -> Result<PathBuf> {
		if let Some(vostok_path) = std::env::var_os("VOSTOK_PATH") {
			let path = PathBuf::from(vostok_path);
			return match path.is_dir() {
				true => Ok(path.join("mods")),
				false => bail!("path is not a directory: {}", path.display()),
			};
		}

		if let Some(path) = path {
			return Ok(path);
		}

		let default_exe = Self::vostok_exe_path();
		if default_exe.is_file() {
			return Self::get_mods_dir_from_exe(&default_exe);
		}

		bail!(
            "install path is required unless VOSTOK_PATH is set or the default Road to Vostok executable exists"
        );
	}

	fn get_mods_dir_from_exe(exe_path: &Path) -> Result<PathBuf> {
		let parent = exe_path.parent().with_context(|| {
			format!(
				"failed to resolve parent directory for {}",
				exe_path.display()
			)
		})?;
		Ok(parent.join("mods"))
	}

	fn replace_macros(template: &str, mod_name: &str) -> String {
		let mod_safe_name = to_safe_name(mod_name);
		let mod_id = to_skewer_case(mod_name);

		template
			.replace("${MOD_NAME}", mod_name)
			.replace("${MOD_SAFE_NAME}", &mod_safe_name)
			.replace("${MOD_ID}", &mod_id)
	}

	fn add_file_to_zip(
		writer: &mut zip::ZipWriter<File>,
		source_path: &Path,
		archive_name: &str,
		compression_method: CompressionMethod,
		used_names: &mut HashSet<String>,
	) -> Result<()> {
		if !used_names.insert(archive_name.to_string()) {
			bail!("duplicate archive entry: {archive_name}");
		}

		let timestamp = Self::resolve_zip_datetime(source_path)?;
		let options = SimpleFileOptions::default()
			.compression_method(compression_method)
			.last_modified_time(timestamp);

		writer
			.start_file(archive_name, options)
			.with_context(|| format!("failed to start zip entry: {archive_name}"))?;

		let mut file = File::open(source_path)
			.with_context(|| format!("failed to open source file: {}", source_path.display()))?;
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer)
			.with_context(|| format!("failed to read source file: {}", source_path.display()))?;
		writer
			.write_all(&buffer)
			.with_context(|| format!("failed to write zip entry: {archive_name}"))?;
		Ok(())
	}

	fn resolve_zip_datetime(source_path: &Path) -> Result<DateTime> {
		let modified = fs::metadata(source_path)
			.and_then(|meta| meta.modified())
			.unwrap_or_else(|_| SystemTime::now());

		Ok(Self::sys_time_to_zip_datetime(modified))
	}

	fn sys_time_to_zip_datetime(system_time: SystemTime) -> DateTime {
		let dt = OffsetDateTime::from(system_time);

		DateTime::from_date_and_time(
			dt.year() as u16,
			dt.month() as u8,
			dt.day(),
			dt.hour(),
			dt.minute(),
			dt.second(),
		)
			.unwrap_or_default()
	}

	fn enforce_extension(mut path: PathBuf, extension: &str) -> PathBuf {
		let ext = path
			.extension()
			.and_then(|e| e.to_str())
			.map(|e| e.eq_ignore_ascii_case(extension))
			.unwrap_or(false);
		if !ext {
			path.set_extension(extension);
		}
		path
	}

	fn path_to_zip_name(path: PathBuf) -> String {
		path.components()
			.map(|c| c.as_os_str().to_string_lossy().to_string())
			.collect::<Vec<_>>()
			.join("/")
	}

	fn vostok_exe_path() -> PathBuf {
		if cfg!(windows) {
			PathBuf::from(Self::WINDOWS_DEFAULT_EXE_PATH)
		} else {
			PathBuf::from(Self::LINUX_DEFAULT_EXE_PATH)
		}.join(Self::default_vostok_exe_name())
	}

	fn default_vostok_exe_name() -> &'static str {
		if cfg!(windows) {
			"RTV.exe"
		} else {
			"RTV.x86_64"
		}
	}

	fn resolve_exe_path(exe_path: Option<PathBuf>) -> Result<PathBuf> {
		if let Some(path) = exe_path {
			if path.is_file() {
				return Ok(path);
			}
			bail!("executable path does not exist or is not a file: {}", path.display());
		}

		if let Some(vostok_path) = std::env::var_os("VOSTOK_PATH") {
			let path = PathBuf::from(vostok_path);
			let candidate = if path.is_dir() {
				path.join(Self::default_vostok_exe_name())
			} else {
				path
			};

			if candidate.is_file() {
				return Ok(candidate);
			}

			bail!(
				"failed to resolve executable from VOSTOK_PATH: {}",
				candidate.display()
			);
		}

		let default_exe = Self::vostok_exe_path();
		if default_exe.is_file() {
			return Ok(default_exe);
		}

		bail!(
			"failed to locate Road to Vostok executable; try setting VOSTOK_PATH"
		)
	}

	fn vostok_data_path() -> Option<PathBuf> {
		match BaseDirs::new() {
			Some(dirs) => {
				let appdata = dirs.data_dir();
				if cfg!(windows) {
					let game_data_path = appdata.join(Self::VOSTOK_APP_NAME);
					Some(game_data_path)
				} else {
					let game_data_path = appdata
						.join("Steam")
						.join("steamapps")
						.join("compatdata")
						.join(Self::VOSTOK_APP_ID.to_string())
						.join("pfx")
						.join("drive_c")
						.join("users")
						.join("steamuser")
						.join("AppData")
						.join("Roaming")
						.join(Self::VOSTOK_APP_NAME);
					Some(game_data_path)
				}
			}
			None => None,
		}
	}

	fn vostok_log_path() -> Option<PathBuf> {
		let vostok_path = Self::vostok_data_path();
		if let Some(path) = vostok_path {
			Some(path.join("logs"))
		} else {
			None
		}
	}

	fn read_log_state(path: &Path) -> Result<(u64, Option<SystemTime>)> {
		let metadata = fs::metadata(path)
			.with_context(|| format!("failed to read metadata for {}", path.display()))?;
		Ok((metadata.len(), metadata.modified().ok()))
	}

	fn print_log(path: &Path) -> Result<()> {
		let bytes = fs::read(path)
			.with_context(|| format!("failed to read log file {}", path.display()))?;
		Self::print_log_bytes(&bytes);
		Ok(())
	}

	fn print_log_from_offset(path: &Path, offset: u64) -> Result<u64> {
		let mut file = File::open(path)
			.with_context(|| format!("failed to open log file {}", path.display()))?;
		file.seek(SeekFrom::Start(offset))
			.with_context(|| format!("failed to seek log file {}", path.display()))?;

		let mut bytes = Vec::new();
		file.read_to_end(&mut bytes)
			.with_context(|| format!("failed to read log file {}", path.display()))?;

		Self::print_log_bytes(&bytes);
		Ok(bytes.len() as u64)
	}

	fn print_log_bytes(bytes: &[u8]) {
		let text = String::from_utf8_lossy(bytes);

		if !std::io::stdout().is_terminal() {
			print!("{}", text);
			return;
		}

		for line in text.split_inclusive('\n') {
			print!("{}", Self::colorize_log_line(line));
		}
	}

	fn colorize_log_line(line: &str) -> String {
		#[allow(unused)]
		enum LogFlagStyle {
			Info,
			Warning,
			Error,
			Trace,
			Stack,
		}

		static RULES: OnceLock<Vec<(Regex, LogFlagStyle)>> = OnceLock::new();
		let rules = RULES.get_or_init(|| {
			vec![
				(Regex::new(r"(?i)^(\[.+\])*\[(info)\]").unwrap(), LogFlagStyle::Info),
				(Regex::new(r"(?i)^(\[.+\])\s*info:").unwrap(), LogFlagStyle::Info),
				(Regex::new(r"(?i)^(\[.+\])*\[(warning)\]").unwrap(), LogFlagStyle::Warning),
				(Regex::new(r"(?i)^(\[.+\])*\s*warning:").unwrap(), LogFlagStyle::Warning),
				(Regex::new(r"(?i)^(\[.+\])*\[(error|critical)\]").unwrap(), LogFlagStyle::Error),
				(Regex::new(r"(?i)^(\[.+\])*\s*(error|critical|script error):").unwrap(), LogFlagStyle::Error),
				(Regex::new(r"(?i)^(\[.+\])*\[(debug)\]").unwrap(), LogFlagStyle::Trace),
				(Regex::new(r"(?i)^(\[.+\])*\s*debug:").unwrap(), LogFlagStyle::Trace),
				(Regex::new(r"^[^\S\r\n]{3,}at:\s+.*").unwrap(), LogFlagStyle::Stack),
			]
		});

		for (rule, style) in rules {
			if let Some(matched) = rule.find(line) {
				if matched.start() != 0 {
					continue;
				}

				let (flag, rest) = line.split_at(matched.end());
				let styled_flag = match style {
					LogFlagStyle::Error => console::style(flag).red().bold(),
					LogFlagStyle::Warning => console::style(flag).yellow().bold(),
					LogFlagStyle::Info => console::style(flag).cyan(),
					LogFlagStyle::Trace => console::style(flag).black().bright(),
					LogFlagStyle::Stack => console::style(flag).black().bright(),
				};
				return format!("{}{}", styled_flag, rest);
			}
		}
		line.to_string()
	}

	fn is_log_event(event: &Event, log_file: &Path) -> bool {
		event.paths.iter().any(|path| {
			path == log_file
				|| path
				.file_name()
				.and_then(|name| name.to_str())
				.map(|name| name.eq_ignore_ascii_case(Self::LOG_NAME))
				.unwrap_or(false)
		})
	}

	fn should_refresh_log(event: &Event, log_file: &Path) -> bool {
		if !Self::is_log_event(event, log_file) {
			return false;
		}

		matches!(
            event.kind,
            EventKind::Create(_)
                | EventKind::Modify(_)
                | EventKind::Remove(_)
                | EventKind::Any
                | EventKind::Other
        )
	}

	fn ctrl_c_flag() -> std::sync::Arc<AtomicBool> {
		static CTRL_C_FLAG: OnceLock<std::sync::Arc<AtomicBool>> = OnceLock::new();
		let flag = CTRL_C_FLAG.get_or_init(|| {
			let flag = std::sync::Arc::new(AtomicBool::new(false));
			let signal_flag = std::sync::Arc::clone(&flag);
			let _ = ctrlc::set_handler(move || {
				signal_flag.store(true, Ordering::SeqCst);
			});
			flag
		});

		std::sync::Arc::clone(flag)
	}

	pub fn log(watch: bool) -> Result<()> {
		let _alt_screen = if watch {
			Some(ScopedTermBuffer::enter()?)
		} else {
			None
		};

		let log_dir =
			Self::vostok_log_path().context("failed to locate Road to Vostok log directory")?;
		let log_file = log_dir.join(Self::LOG_NAME);

		if !log_file.is_file() {
			bail!("log file not found: {}", log_file.display());
		}

		print_status("Reading", log_file.display().to_string());
		Self::print_log(&log_file)?;

		if !watch {
			return Ok(());
		}

		let (tx, rx) = mpsc::channel();
		let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |result| {
			let _ = tx.send(result);
		})
			.context("failed to initialize filesystem watcher")?;
		watcher
			.watch(&log_dir, RecursiveMode::NonRecursive)
			.with_context(|| format!("failed to watch {}", log_dir.display()))?;

		print_status(
			"Watching",
			format!("{} (Ctrl+C to stop)", log_file.display()),
		);

		let ctrl_c_flag = Self::ctrl_c_flag();
		ctrl_c_flag.store(false, Ordering::SeqCst);

		let mut read_offset = fs::metadata(&log_file).map(|meta| meta.len()).unwrap_or(0);
		let mut was_missing = false;

		loop {
			if ctrl_c_flag.load(Ordering::SeqCst) {
				break;
			}

			match rx.recv_timeout(Duration::from_millis(250)) {
				Err(mpsc::RecvTimeoutError::Timeout) => continue,
				Err(mpsc::RecvTimeoutError::Disconnected) => {
					bail!("filesystem watcher channel disconnected")
				}
				Ok(Ok(event)) => {
					if !Self::should_refresh_log(&event, &log_file) {
						continue;
					}

					match Self::read_log_state(&log_file) {
						Ok((len, _)) => {
							if len < read_offset {
								// File was truncated/rotated; start reading from beginning again.
								read_offset = 0;
							}

							if len > read_offset || was_missing {
								was_missing = false;
								println!();
								print_status("Updated", log_file.display().to_string());

								let appended = Self::print_log_from_offset(&log_file, read_offset)?;
								read_offset += appended;
							}
						}
						Err(_) => {
							read_offset = 0;
							if !was_missing {
								print_status(
									"Info",
									format!(
										"log file is temporarily unavailable, waiting for {}",
										log_file.display()
									),
								);
								was_missing = true;
							}
						}
					}
				}
				Ok(Err(err)) => {
					print_status("Info", format!("watch error: {err}"));
				}
			}
		}

		Ok(())
	}

	pub fn run(exe_path: Option<PathBuf>, api: Option<RenderingAPI>, args: Vec<String>) -> Result<()> {
		let exe_path = Self::resolve_exe_path(exe_path)?;
		let mut launch_args = args;
		if let Some(api) = api {
			launch_args.push("--rendering-driver".to_string());
			launch_args.push(api.as_driver_name().to_string());
		}

		let display_cmd = if launch_args.is_empty() {
			exe_path.display().to_string()
		} else {
			format!("{} {}", exe_path.display(), launch_args.join(" "))
		};
		print_status("Running", display_cmd);

		GameWrapper::new(exe_path, launch_args).run(Self::ctrl_c_flag(), Self::print_log_bytes)?;
		Ok(())
	}

	pub fn modify(mod_path: &Path, mod_info: &mut ModInfo, mod_info_override: ModInfoOverride, silent: bool, overwrite: bool) -> Result<()> {
		let info_path = mod_path.join("mod.txt");
		if overwrite && !info_path.is_file() {
			bail!("info file does not exist");
		}
		let mut changes = 0;

		if let Some(name) = mod_info_override.name {
			if !silent {
				print_status("Changing", format!("name: {} -> {name}", mod_info.base.name));
			}
			changes += 1;
			mod_info.base.name = name;
		}
		if let Some(id) = mod_info_override.id {
			if !silent {
				print_status("Changing", format!("id: {} -> {id}", mod_info.base.id));
			}
			changes += 1;
			mod_info.base.id = id;
		}
		if mod_info_override.priority.is_some() {
			let priority_str = match mod_info.base.priority {
				Some(priority) => priority.to_string(),
				None => "null".to_string()
			};
			if !silent {
				print_status("Changing", format!("priority: {} -> {}", priority_str, mod_info_override.priority.unwrap()));
			}
			changes += 1;
			mod_info.base.priority = mod_info_override.priority;
		}
		if let Some(version) = mod_info_override.version {
			if !silent {
				print_status("Changing", format!("version: {} -> {version}", mod_info.base.version));
			}
			changes += 1;
			mod_info.base.version = version;
		}
		if mod_info_override.updates.is_some() {
			let updates = mod_info_override.updates.unwrap();
			if !silent {
				let modworkshop_str = match mod_info.updates {
					Some(ref updates) => updates.modworkshop.to_string(),
					None => "null".to_string(),
				};
				print_status("Changing", format!("modworkshop: {} -> {}", modworkshop_str, updates.modworkshop));
			}
			changes += 1;
			mod_info.updates = Some(updates);
		}
		if changes > 0 && overwrite {
			if !silent {
				print_status("Applied", format!("{changes} changes to {}", info_path.display()));
			}
			mod_info.write(&info_path)
		} else {
			if !silent {
				print_status("Done", format!("No changes applied to {}", info_path.display()));
			}
			Ok(())
		}
	}
}
