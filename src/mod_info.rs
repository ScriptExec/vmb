use std::cmp::Ordering;
use crate::util::{to_safe_name, to_skewer_case};
use anyhow::{Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct ModInfoOverride {
	pub name: Option<String>,
	pub id: Option<String>,
	pub version: Option<Version>,
	pub priority: Option<i32>,
	pub autoload: Option<HashMap<String, String>>,
	pub updates: Option<ModUpdatesInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModInfo {
	#[serde(rename = "mod")]
	pub base: ModBaseInfo,
	pub autoload: Option<HashMap<String, String>>,
	pub updates: Option<ModUpdatesInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ModBaseInfo {
	pub name: String,
	pub id: String,
	pub version: Version,
	pub priority: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModUpdatesInfo {
	pub modworkshop: u32,
}

impl Eq for ModInfo {}

impl PartialEq for ModInfo {
	fn eq(&self, other: &ModInfo) -> bool {
		self.base == other.base
	}
}

impl Ord for ModInfo {
	fn cmp(&self, other: &Self) -> Ordering {
		self.base.cmp(&other.base)
	}
}

impl PartialOrd for ModInfo {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.base.cmp(&other.base))
	}
}

impl Display for ModBaseInfo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} {}", self.name, self.version)
	}
}

impl Display for ModInfo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.base)
	}
}

impl ModInfo {
	pub fn default_from(mod_name: String) -> Self {
		let mod_safe_name = to_safe_name(&mod_name);
		let mod_id = to_skewer_case(&mod_name);

		Self {
			base: ModBaseInfo {
				name: mod_name.clone(),
				id: mod_id.clone(),
				version: Version::new(0, 1, 0),
				priority: None,
			},
			autoload: Some(HashMap::from([
				(
					mod_safe_name.clone(),
					format!("res://mods/{mod_safe_name}/Main.gd"),
				),
			])),
			updates: None,
		}
	}

	pub fn from_path(path: &Path) -> Result<Self> {
		let content = std::fs::read_to_string(path)
			.with_context(|| format!("failed to read mod info from {}", path.display()))?;
		Self::from_str(&content)
	}

	pub fn from_str(content: &str) -> Result<Self> {
		let info = toml::from_str(content)
			.context("failed to parse mod info")?;
		Ok(info)
	}

	pub fn from_archive(path: &Path) -> Result<ModInfo> {
		let file = File::open(path)
			.with_context(|| format!("failed to open archive {}", path.display()))?;
		let mut archive = ZipArchive::new(file)
			.with_context(|| format!("failed to read zip archive {}", path.display()))?;

		let mut mod_txt = archive
			.by_name("mod.txt")
			.with_context(|| format!("mod.txt not found in {}", path.display()))?;

		let mut content = String::new();
		mod_txt
			.read_to_string(&mut content)
			.with_context(|| format!("failed to read mod.txt in {}", path.display()))?;

		ModInfo::from_str(&content)
			.with_context(|| format!("failed to parse mod.txt in {}", path.display()))
	}

	pub fn to_string(&self) -> Result<String> {
		let content = toml::to_string(&self)?;
		Ok(content)
	}

	pub fn write(&self, path: &Path) -> Result<()> {
		let content = self.to_string()?;
		std::fs::create_dir_all(path.parent().unwrap())?;
		std::fs::write(path, &content)?;
		Ok(())
	}
}
