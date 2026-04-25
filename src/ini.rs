pub fn ini_to_toml(ini: &str) -> String {
	ini.lines()
		.map(|line| {
			if let Some((k, v)) = line.split_once('=') {
				let k = k.trim();
				if k.starts_with('[') || k.starts_with('"') {
					line.to_string()
				} else if k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
					line.to_string()
				} else {
					format!("\"{k}\" = {v}")
				}
			} else {
				line.to_string()
			}
		})
		.collect::<Vec<_>>()
		.join("\n")
}

pub fn toml_to_ini(toml: &str) -> String {
	toml.lines()
		.map(|line| {
			if let Some((k, v)) = line.split_once('=') {
				let k = k.trim();
				let v = v.trim();

				if k.starts_with('[') && k.ends_with(']') {
					line.to_string()
				} else if k.starts_with('"') && k.ends_with('"') && k.len() >= 2 {
					format!("{} = {}", &k[1..k.len() - 1], v)
				} else {
					format!("{k} = {v}")
				}
			} else {
				line.to_string()
			}
		})
		.collect::<Vec<_>>()
		.join("\n")
}
