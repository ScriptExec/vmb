use std::collections::HashMap;

pub struct ModInfo {
	pub name: String,
	pub id: String,
	pub version: String,
	pub priority: Option<u32>,
	pub autoload: Option<ModAutoloadInfo>,
	pub updates: Option<ModUpdatesInfo>,
}

pub struct ModAutoloadInfo {
	pub data: HashMap<String, String>
}

pub struct ModUpdatesInfo {
	pub modworkshop: u32,
}
