use std::fs;
use std::path::PathBuf;

use directories::BaseDirs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use getset::Getters;

#[derive(Serialize, Deserialize, Debug, Default, Getters)]
pub struct AppData {
	#[getset(get = "pub")]
	game_path: Option<PathBuf>,

	#[serde(skip)]
	#[getset(skip)]
	is_dirty: bool,
}

impl AppData {
	const APP_NAME: &'static str = env!("CARGO_PKG_NAME");
	const CONFIG_FILENAME: &'static str = "config.toml";
	const BACKUP_DIR: &'static str = "backup";
	const BACKUP_EXTENSION: &'static str = "bak";

	pub fn new() -> Self {
		AppData::load().unwrap_or_default()
	}

	pub fn load() -> Result<Self> {
		let config_path = Self::config_path().context("failed to determine app config path")?;
		if !config_path.exists() {
			return Ok(Self::default());
		}

		let config_str = fs::read_to_string(config_path)
			.context("failed to read app config file")?;
		let config = toml::from_str(&config_str)
			.context("failed to parse app config file")?;
		Ok(config)
	}

	pub fn save(&self) -> Result<()> {
		if !self.is_dirty {
			return Ok(());
		}

		let data_path = Self::data_path().context("failed to determine app data path")?;
		if !data_path.exists() {
			fs::create_dir_all(&data_path)?;
		}
		let config = toml::to_string(&self)?;
		if !config.is_empty() {
			let out_path = data_path.join(Self::CONFIG_FILENAME);
			fs::write(out_path, config).context("failed to write app config file")
		} else {
			Ok(())
		}
	}

	pub fn backup(&self, path: PathBuf) -> Result<PathBuf> {
		if path.exists() {
			let backup_dir = Self::backup_dir_path()?;
			if !backup_dir.exists() {
				fs::create_dir_all(&backup_dir)?;
			}

			let file_name = path.file_name()
				.and_then(|n| n.to_str())
				.map(str::to_owned)
				.with_context(|| format!("invalid file name: {}", path.display()))?;

			let backup_path = backup_dir.join(format!("{}.{}", file_name, Self::BACKUP_EXTENSION));
			fs::copy(path, &backup_path)
				.map(|_| ())
				.context("failed to create backup")?;
			Ok(backup_path)
		} else {
			Err(anyhow::anyhow!("backup source path does not exist: {}", path.display()))
		}
	}

	pub fn set_game_path(&mut self, value: Option<PathBuf>) {
		if self.game_path != value {
			self.game_path = value;
			self.is_dirty = true;
		}
	}

	fn backup_dir_path() -> Result<PathBuf> {
		let data_path = Self::data_path().context("failed to determine app data path")?;
		Ok(data_path.join(Self::BACKUP_DIR))
	}

	fn config_path() -> Result<PathBuf> {
		let data_path = Self::data_path().context("failed to determine app data path")?;
		Ok(data_path.join(Self::CONFIG_FILENAME))
	}

	fn data_path() -> Option<PathBuf> {
		BaseDirs::new().map(|dirs| {
			let path = dirs.data_dir().join(Self::APP_NAME);
			path
		})
	}
}

impl Drop for AppData {
	fn drop(&mut self) {
		let _ = self.save();
	}
}
