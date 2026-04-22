use crate::util::{to_safe_name, to_skewer_case};
use anyhow::{Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
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
		let info = toml::from_str(&content)
			.with_context(|| format!("failed to parse mod info from {}", path.display()))?;
		Ok(info)
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
