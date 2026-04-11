use anyhow::Context;
use console::style;
use std::fs;
use std::path::Path;

pub fn derive_dir_name(path: &Path) -> anyhow::Result<String> {
	let base_dir = if is_working_dir(path) {
		std::env::current_dir().context("failed to read current directory")?
	} else {
		path.to_path_buf()
	};

	base_dir
		.file_name()
		.and_then(|name| name.to_str())
		.map(str::to_owned)
		.filter(|name| !name.trim().is_empty())
		.with_context(|| format!("could not derive mod name from path: {}", path.display()))
}

pub fn to_safe_name(name: &str) -> String {
	let mut out = String::new();
	for c in name.chars() {
		if c.is_ascii_whitespace() {
			continue;
		}

		if c.is_ascii_alphanumeric() || c == '_' {
			out.push(c);
		} else {
			out.push('_');
		}
	}

	if out.is_empty() {
		return "unknown".to_string();
	}

	if out.starts_with(|c: char| c.is_ascii_digit()) {
		out.insert(0, '_');
	}

	out
}

pub fn to_skewer_case(name: &str) -> String {
	name.split_whitespace()
		.map(|part| part.to_ascii_lowercase())
		.collect::<Vec<_>>()
		.join("-")
}

pub fn is_working_dir(path: &Path) -> bool {
	if path == Path::new(".") {
		return true;
	}

	let cwd = match std::env::current_dir() {
		Ok(dir) => dir,
		Err(_) => return false,
	};

	match (fs::canonicalize(path), fs::canonicalize(cwd)) {
		(Ok(lhs), Ok(rhs)) => lhs == rhs,
		_ => false,
	}
}

pub fn print_status(status: &str, message: impl AsRef<str>) {
	println!("{:>12} {}", style(status).green().bold(), message.as_ref());
}

pub fn print_error(message: impl AsRef<str>) {
	eprintln!("{} {}", style("error:").red().bold(), message.as_ref());
}

