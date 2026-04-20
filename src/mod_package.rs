use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{bail, Context, Result};
use indicatif::ProgressBar;
use time::OffsetDateTime;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime};

use crate::mod_info::ModInfo;
use crate::progress::default_progress_style;
use crate::util::print_status;

pub struct ModPackage {
	inputs: Vec<PathBuf>,
	pub info: ModInfo,
}

impl ModPackage {
	pub fn new(info: ModInfo) -> Self {
		Self {
			inputs: Vec::new(),
			info,
		}
	}

	pub fn pack(&self, output: PathBuf) -> Result<()> {
		let output = Self::enforce_extension(output, "vmz");

		if self.is_empty() {
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
		let progress = Self::new_pack_progress(self.count_packable_bytes()?);

		for input in &self.inputs {
			if !input.exists() {
				bail!("input path does not exist: {}", input.display());
			}

			if input.is_file() {
				let file_name = input
					.file_name()
					.and_then(|n| n.to_str())
					.map(str::to_owned)
					.with_context(|| format!("invalid file name: {}", input.display()))?;

				Self::pack_file(
					&mut writer,
					&input,
					&file_name,
					CompressionMethod::Deflated,
					&mut used_names,
				)?;
				if let Some(pb) = progress.as_ref() {
					let written = fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
					pb.inc(written);
				}
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
				let archive_name = Self::normalize_path(Path::new(&root_name).join(rel));
				Self::pack_file(
					&mut writer,
					path,
					&archive_name,
					CompressionMethod::Deflated,
					&mut used_names,
				)?;
				if let Some(pb) = progress.as_ref() {
					let written = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
					pb.inc(written);
				}
			}
		}

		writer.finish().context("failed to finalize archive")?;
		if let Some(pb) = progress {
			pb.finish_and_clear();
		}
		print_status("Created", output.display().to_string());
		Ok(())
	}

	fn count_packable_bytes(&self) -> Result<u64> {
		let mut total = 0_u64;

		for input in &self.inputs {
			if !input.exists() {
				bail!("input path does not exist: {}", input.display());
			}

			if input.is_file() {
				total += fs::metadata(input)
					.with_context(|| format!("failed to read metadata for {}", input.display()))?
					.len();
				continue;
			}

			for entry in WalkDir::new(input).into_iter().filter_map(|e| e.ok()) {
				if entry.path().is_file() {
					total += fs::metadata(entry.path())
						.with_context(|| {
							format!("failed to read metadata for {}", entry.path().display())
						})?
						.len();
				}
			}
		}

		Ok(total)
	}

	fn new_pack_progress(total_bytes: u64) -> Option<ProgressBar> {
		if total_bytes == 0 || !std::io::stdout().is_terminal() {
			return None;
		}

		let pb = ProgressBar::new(total_bytes);
		pb.set_style(default_progress_style());
		pb.enable_steady_tick(std::time::Duration::from_millis(100));
		Some(pb)
	}

	pub fn set_files(&mut self, inputs: Vec<PathBuf>) {
		self.inputs = inputs;
	}

	fn pack_file(
		writer: &mut zip::ZipWriter<File>,
		source_path: &Path,
		archive_name: &str,
		compression_method: CompressionMethod,
		used_names: &mut HashSet<String>,
	) -> Result<()> {
		if !used_names.insert(archive_name.to_string()) {
			bail!("duplicate archive entry: {archive_name}");
		}

		let mod_time = Self::get_file_mod_time(source_path)?;
		let options = SimpleFileOptions::default()
			.compression_method(compression_method)
			.last_modified_time(mod_time);

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

	fn get_file_mod_time(source_path: &Path) -> Result<DateTime> {
		let modified = fs::metadata(source_path)
			.and_then(|meta| meta.modified())
			.unwrap_or(SystemTime::now());

		Ok(Self::sys_time_to_datetime(modified))
	}

	fn sys_time_to_datetime(system_time: SystemTime) -> DateTime {
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

	fn normalize_path(path: PathBuf) -> String {
		path.components()
			.map(|c| c.as_os_str().to_string_lossy().to_string())
			.collect::<Vec<_>>()
			.join("/")
	}

	pub fn is_empty(&self) -> bool {
		self.inputs.is_empty()
	}
}
