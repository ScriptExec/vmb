use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use semver::Version;
use crate::ini::{ini_to_toml, toml_to_ini};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModConfig {
	pub settings: ModConfigSettings,
	pub profile: HashMap<String, ModConfigProfile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModConfigSettings {
	pub developer_mode: bool,
	pub active_profile: String,
	#[serde(flatten)]
	pub rest: HashMap<String, toml::Value>,
}

impl Default for ModConfigSettings {
	fn default() -> Self {
		Self {
			developer_mode: false,
			active_profile: "Default".to_string(),
			rest: HashMap::new(),
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ModConfigProfile {
	pub enabled: HashMap<String, bool>,
	pub priority: HashMap<String, i32>,
}

impl ModConfigProfile {
	pub fn clear(&mut self) {
		self.enabled.clear();
		self.priority.clear();
	}

	pub fn to_entries(&self, id_map: Option<HashMap<String, String>>) -> Result<Vec<ModConfigEntry>> {
		let mut entries = self.enabled
			.iter()
			.map(|(mod_name, enabled)| {
				let priority = self.priority.get(mod_name)
					.cloned()
					.unwrap_or(0);

				let mut entry = if let Some((name, version)) = mod_name.split_once('@') {
					let semver = Version::parse(&version.to_string())
						.context("invalid version format in mod config entry")
						.unwrap();

					ModConfigEntry {
						id: name.to_string(),
						version: Some(semver),
						name: None,
						enabled: *enabled,
						priority,
					}
				} else {
					ModConfigEntry {
						id: mod_name.clone(),
						version: None,
						name: None,
						enabled: *enabled,
						priority,
					}
				};
				if let Some(map) = &id_map {
					if let Some(mapped_name) = map.get(&entry.id) {
						entry.name = Some(mapped_name.clone());
					}
				}
				entry
			})
			.collect::<Vec<_>>();

		entries.sort();
		Ok(entries)
	}

	pub fn set_entries(&mut self, entries: Vec<ModConfigEntry>) {
		self.clear();
		self.clear();

		for entry in entries {
			self.enabled.insert(entry.name(), entry.enabled);
			self.priority.insert(entry.name(), entry.priority);
		}
	}
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ModConfigEntry {
	pub id: String,
	pub version: Option<Version>,
	pub name: Option<String>,
	pub priority: i32,
	pub enabled: bool,
}

impl Display for ModConfigEntry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(name) = &self.name {
			if let Some(version) = &self.version {
				write!(f, "{} {}", name, version)
			} else {
				write!(f, "{}", name)
			}
		} else if let Some(version) = &self.version {
			write!(f, "{} {}", self.id, version)
		} else {
			write!(f, "{}", self.id)
		}
	}}

impl ModConfigEntry {
	pub fn name(&self) -> String {
		if let Some(version) = &self.version {
			format!("{}@{}", self.id, version)
		} else {
			self.id.clone()
		}
	}
}

impl ModConfig {
	pub fn from_path(data_path: &PathBuf) -> Result<ModConfig> {
		let ini = std::fs::read_to_string(&data_path).context("failed to read mod config file")?;
		let toml = ini_to_toml(ini.as_str());
		let deserialized: ModConfig = toml::from_str(&toml)?;
		Ok(deserialized)
	}

	pub fn active_profile(&mut self) -> Option<(String, &mut ModConfigProfile)> {
		if !self.settings.active_profile.is_empty() {
			let active_profile = self.settings.active_profile.clone();
			let profile = self.profile.get_mut(&active_profile)?;
			Some((active_profile, profile))
		} else {
			None
		}
	}

	pub fn to_string(&self) -> Result<String> {
		let ini = toml_to_ini(toml::to_string(self)?.as_str());
		Ok(ini)
	}

	pub fn write(&self, path: &Path) -> Result<()> {
		let content = self.to_string()?;
		std::fs::write(path, content).context("failed to write mod config file")
	}
}
