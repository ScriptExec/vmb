use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const TEMPLATES_PATH: &str = "templates";

fn main() {
    let templates_path = Path::new(TEMPLATES_PATH);
    println!("cargo:rerun-if-changed={}", templates_path.display());

    let files = collect_template_files(templates_path);
    let generated = emit_template_map(&files, templates_path);

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is not set by Cargo");
    let out_file = Path::new(&out_dir).join("generated_templates.rs");
    fs::write(&out_file, generated)
        .unwrap_or_else(|err| panic!("failed writing {}: {err}", out_file.display()));
}

fn collect_template_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_template_files_inner(root, &mut files);
    files.sort();
    files
}

fn collect_template_files_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed reading directory {}: {err}", dir.display()));

    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("failed reading directory entry: {err}"));
        let path = entry.path();

        if path.is_dir() {
            collect_template_files_inner(&path, out);
            continue;
        }

        if path.is_file() {
            out.push(path);
        }
    }
}

fn emit_template_map(files: &[PathBuf], root: &Path) -> String {
    let mut out = String::from(
        "pub fn template_map() -> std::collections::HashMap<&'static str, &'static str> {\n",
    );
    out.push_str("    let mut map = std::collections::HashMap::new();\n");

    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
        let key = template_key(root, file);
        let content = fs::read_to_string(file)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", file.display()));
        out.push_str(&format!("    map.insert({key:?}, {content:?});\n"));
    }

    out.push_str("    map\n");
    out.push_str("}\n");
    out
}

fn template_key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or_else(|_| panic!("failed to strip template root from {}", path.display()))
        .to_string_lossy()
        .replace('\\', "/")
}

