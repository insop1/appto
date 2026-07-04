use anyhow::{bail, Context, Result};
use std::path::{PathBuf, Path};
use std::io::{self, Write};
use std::process::Command;
use crate::paths;
use crate::desktop;
use std::fs;
use std::os::unix::fs::PermissionsExt;

pub fn add(path: PathBuf, edit: bool, force: bool) -> Result<()> {
    // Extract appimage
    // Check the name from desktop entry
    // If it already exists, asks to overwrite.

    let cache_dir = paths::appto_cache()?;
    let data_dir = paths::appto_data()?; 
    let squash_dir = cache_dir.join("squashfs-root");
    paths::ensure_paths(&[&cache_dir, &data_dir])?;

    extract_appimage(&path, &cache_dir, &squash_dir)?;
    // By now we should have squashfs-root set
    let dir_icon = squash_dir.join(".DirIcon");
    let desktop = fs::read_dir(&squash_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|ext| ext == "desktop"))
        .context("Could not find .desktop in AppImage")?;

    // Overwrite check then lock
    let metadata = desktop::desktop_metadata(&desktop); 
    if edit {
        if let Err(e) = desktop::edit_desktop(&desktop) {
            eprintln!("warning: {:#}, continuing with defaults", e);
        }
    }

    Ok(())
}

fn extract_appimage(appimage: &Path, cache_dir: &Path, squash_dir: &Path) -> Result<()> {
    let appimage = appimage.canonicalize().context("Failed to resolve AppImage path")?;
    if squash_dir.exists() {
        fs::remove_dir_all(&squash_dir).context("Failed to clear squashfs-root")?;
    }

    let mut perms = fs::metadata(&appimage)
        .context("Failed to read AppImage metadata")?
        .permissions();

    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(&appimage, perms).context("Failed to make AppImage executable")?;

    let status = Command::new(appimage)
        .arg("--appimage-extract")
        .current_dir(cache_dir)
        .stdout(std::process::Stdio::null())
        .status()
        .context("Failed to extract AppImage")?;

    if !status.success() {
        bail!("extraction failed with {}", status);
    }

    Ok(())
}
