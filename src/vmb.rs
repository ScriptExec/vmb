use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::util::{derive_dir_name, to_safe_name, to_skewer_case};
use anyhow::{bail, Context, Result};
use git2::Repository;
use tempfile::TempDir;
use time::OffsetDateTime;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::DateTime;

include!(concat!(env!("OUT_DIR"), "/generated_templates.rs"));

pub struct Vmb;

impl Vmb {
    const WINDOWS_DEFAULT_EXE_PATH: &str =
        "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Road to Vostok\\RTV.exe";

    fn load_template(name: &str) -> Result<&str> {
        let templates = template_map();
        templates
            .get(name)
            .with_context(|| format!("Template not found: {}", name))
            .copied()
    }

    pub fn init(mod_path: PathBuf, no_git: bool, _update_id: Option<u32>) -> Result<()> {
        let mod_name = derive_dir_name(&mod_path)?;
        let mod_safe_name = to_safe_name(&mod_name);

        let mod_txt_template = Self::load_template("mod_txt")?;
        let main_gd_template = Self::load_template("Main.gd")?;

        fs::create_dir_all(&mod_path).with_context(|| {
            format!(
                "failed to create or access mod directory: {}",
                mod_path.display()
            )
        })?;

        let mod_txt_path = mod_path.join("mod.txt");
        let main_gd_path = mod_path.join("mods").join(&mod_safe_name).join("Main.gd");

        if mod_txt_path.exists() {
            bail!(
                "refusing to overwrite existing file: {}",
                mod_txt_path.display()
            );
        }
        if main_gd_path.exists() {
            bail!(
                "refusing to overwrite existing file: {}",
                main_gd_path.display()
            );
        }

        if let Some(parent) = main_gd_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        fs::write(
            &mod_txt_path,
            Self::replace_macros(mod_txt_template, &mod_name),
        )
        .with_context(|| format!("failed to write {}", mod_txt_path.display()))?;
        fs::write(
            &main_gd_path,
            Self::replace_macros(main_gd_template, &mod_name),
        )
        .with_context(|| format!("failed to write {}", main_gd_path.display()))?;

        if no_git {
            println!("Skipping git repository initialization (\"--no-git\" specified)");
        } else {
            Self::init_git_repo(&mod_path)?;
        }

        println!("Initialized mod '{}' at {}", mod_name, mod_path.display());
        Ok(())
    }

    fn init_git_repo(mod_path: &Path) -> Result<()> {
        let git_dir = mod_path.join(".git");

        if git_dir.is_dir() {
            return Ok(());
        }
        if git_dir.exists() {
            bail!(".git exists but is not a directory: {}", git_dir.display());
        }

        Repository::init(mod_path).with_context(|| {
            format!(
                "failed to initialize git repository in {}",
                mod_path.display()
            )
        })?;
        Ok(())
    }

    pub fn pack(output: PathBuf, inputs: Vec<PathBuf>) -> Result<()> {
        let output = Self::enforce_extension(output, "vmz");

        if inputs.is_empty() {
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

        for input in inputs {
            if !input.exists() {
                bail!("input path does not exist: {}", input.display());
            }

            if input.is_file() {
                let file_name = input
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_owned)
                    .with_context(|| format!("invalid file name: {}", input.display()))?;

                Self::add_file_to_zip(
                    &mut writer,
                    &input,
                    &file_name,
                    CompressionMethod::Deflated,
                    &mut used_names,
                )?;
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
                let archive_name = Self::path_to_zip_name(Path::new(&root_name).join(rel));
                Self::add_file_to_zip(
                    &mut writer,
                    path,
                    &archive_name,
                    CompressionMethod::Deflated,
                    &mut used_names,
                )?;
            }
        }

        writer.finish().context("failed to finalize archive")?;
        println!("Created {}", output.display());
        Ok(())
    }

    pub fn install(source: PathBuf, path: Option<PathBuf>) -> Result<()> {
        let (archive, _temp_dir) = Self::prepare_install_source(source)?;

        let target_dir = Self::resolve_install_dir(path)?;
        fs::create_dir_all(&target_dir).with_context(|| {
            format!(
                "failed to create install directory: {}",
                target_dir.display()
            )
        })?;

        Self::install_archive(&archive, &target_dir)
    }

    fn prepare_install_source(source: PathBuf) -> Result<(PathBuf, Option<TempDir>)> {
        if source.is_file() {
            let ext = source
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();
            if ext != "zip" && ext != "vmz" {
                bail!("archive must be .zip or .vmz: {}", source.display());
            }
            return Ok((source, None));
        }

        if !source.is_dir() {
            bail!(
                "source does not exist or is not a file/directory: {}",
                source.display()
            );
        }

        if !Self::is_mod_root(&source) {
            bail!(
                "directory is not a mod root (expected mod.txt and mods/): {}",
                source.display()
            );
        }

        let mod_name = derive_dir_name(&source)?;
        let temp_dir = TempDir::new().context("failed to create temporary directory")?;
        let temp_archive = temp_dir
            .path()
            .join(format!("{}.vmz", to_safe_name(&mod_name)));
        let inputs = vec![source.join("mod.txt"), source.join("mods")];
        Self::pack(temp_archive.clone(), inputs)
            .with_context(|| format!("failed to package mod root {}", source.display()))?;

        Ok((temp_archive, Some(temp_dir)))
    }

    fn install_archive(archive: &Path, target_dir: &Path) -> Result<()> {
        let file_name = archive
            .file_name()
            .context("archive path has no file name")?;
        let destination = target_dir.join(file_name);

        fs::copy(&archive, &destination).with_context(|| {
            format!(
                "failed to copy {} to {}",
                archive.display(),
                destination.display()
            )
        })?;

        println!("Installed {}", destination.display());
        Ok(())
    }

    /// Expects ./mod.txt and ./mods dir to exist, but does not validate their contents
    fn is_mod_root(path: &Path) -> bool {
        path.join("mod.txt").is_file() && path.join("mods").is_dir()
    }

    fn resolve_install_dir(path: Option<PathBuf>) -> Result<PathBuf> {
        if let Some(vostok_path) = std::env::var_os("VOSTOK_PATH") {
            let path = PathBuf::from(vostok_path);
            return match path.is_dir() {
                true => Ok(path),
                false => bail!("path is not a directory: {}", path.display()),
            }
        }

        if let Some(path) = path {
            return Ok(path);
        }

        if cfg!(windows) {
            let default_exe = PathBuf::from(Self::WINDOWS_DEFAULT_EXE_PATH);
            if default_exe.is_file() {
                return Self::get_mods_dir_from_exe(&default_exe);
            }
        }

        bail!(
            "install path is required unless VOSTOK_PATH is set or the default Road to Vostok executable exists"
        );
    }

    fn get_mods_dir_from_exe(exe_path: &Path) -> Result<PathBuf> {
        let parent = exe_path.parent().with_context(|| {
            format!(
                "failed to resolve parent directory for {}",
                exe_path.display()
            )
        })?;
        Ok(parent.join("mods"))
    }

    fn replace_macros(template: &str, mod_name: &str) -> String {
        let mod_safe_name = to_safe_name(mod_name);
        let mod_id = to_skewer_case(mod_name);

        template
            .replace("${MOD_NAME}", mod_name)
            .replace("${MOD_SAFE_NAME}", &mod_safe_name)
            .replace("${MOD_ID}", &mod_id)
    }

    fn add_file_to_zip(
        writer: &mut zip::ZipWriter<File>,
        source_path: &Path,
        archive_name: &str,
        compression_method: CompressionMethod,
        used_names: &mut HashSet<String>,
    ) -> Result<()> {
        if !used_names.insert(archive_name.to_string()) {
            bail!("duplicate archive entry: {archive_name}");
        }

        let timestamp = Self::resolve_zip_datetime(source_path)?;
        let options = SimpleFileOptions::default()
            .compression_method(compression_method)
            .last_modified_time(timestamp);

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

    fn resolve_zip_datetime(source_path: &Path) -> Result<DateTime> {
        let modified = fs::metadata(source_path)
            .and_then(|meta| meta.modified())
            .unwrap_or_else(|_| SystemTime::now());

        Ok(Self::sys_time_to_zip_datetime(modified))
    }

    fn sys_time_to_zip_datetime(system_time: SystemTime) -> DateTime {
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

    fn path_to_zip_name(path: PathBuf) -> String {
        path.components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/")
    }
}
