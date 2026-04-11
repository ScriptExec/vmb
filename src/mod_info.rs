use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ModInfo {
	pub name: String,
	pub id: String,
	pub version: String,
	pub priority: Option<u32>,
	pub autoload: Option<ModAutoloadInfo>,
	pub updates: Option<ModUpdatesInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct ModAutoloadInfo {
	pub data: HashMap<String, String>
}

#[derive(Serialize, Deserialize)]
pub struct ModUpdatesInfo {
	pub modworkshop: u32,
}
