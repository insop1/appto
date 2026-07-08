use anyhow::{Context, Result, bail};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use crate::container;
use crate::container::Container;
use crate::desktop::{self, DesktopMetadata};
use crate::paths;

pub fn add(appimage: &Path, force: bool) -> Result<()> {
    let cache_dir = paths::appto_cache()?;
    let data_dir = paths::appto_data()?;
    let squash_dir = cache_dir.join("squashfs-root");
    paths::ensure_paths(&[&cache_dir, &data_dir])?;

    extract_appimage(appimage, &cache_dir, &squash_dir)?;
    // By now we should have squashfs-root set
    let desktop = desktop::desktop_from(&squash_dir)?;
    let metadata = desktop::desktop_metadata(&desktop)?;

    // Overwrite check then lock
    let container = Container::new(&data_dir, metadata.slug());
    if !force && container.root().is_dir() && !confirm_overwrite(&container, &metadata)? {
        println!("Aborted.");
        return Ok(());
    }

    container.create()?;
    let original_content = fs::read_to_string(&desktop)?;
    let new_content = desktop::updated_desktop(&original_content, &container)?;
    let dir_icon = squash_dir.join(".DirIcon");
    container.install(&new_content, &dir_icon, appimage)?;

    // Removes squashfs-root after installing container
    if let Err(e) = fs::remove_dir_all(squash_dir) {
        eprintln!("Could not remove squashfs-root temp file: {e:#}");
    }

    let application_dir = paths::application_dir()?;
    if let Err(e) = container.symlink_desktop(&application_dir) {
        eprintln!("Warning: {e:#}");
    }

    let bin_dir = paths::bin_dir()?;
    if let Err(e) = container.symlink_appimage(&bin_dir) {
        eprintln!("Warning: {e:#}");
    }

    paths::warn_path_missing(&bin_dir);
    print!("Successfully installed {}", metadata.name());
    println_version_or(metadata.version(), ".");
    Ok(())
}

fn extract_appimage(appimage: &Path, cache_dir: &Path, squash_dir: &Path) -> Result<()> {
    let appimage = appimage
        .canonicalize()
        .context("Failed to resolve AppImage path")?;
    if squash_dir.exists() {
        fs::remove_dir_all(squash_dir).context("Failed to clear squashfs-root")?;
    }

    let mut perms = fs::metadata(&appimage)
        .context("Failed to read AppImage metadata")?
        .permissions();

    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(&appimage, perms).context("Failed to make AppImage executable")?;

    // Dumb extraction pattern
    let patterns = [
        "*.desktop",
        "*.png",
        "*.svg",
        ".DirIcon",
        "usr/share/icons/*",
        "usr/share/pixmaps/*",
    ];
    for pattern in patterns {
        let status = Command::new(&appimage)
            .arg("--appimage-extract")
            .arg(pattern)
            .current_dir(cache_dir)
            .stdout(std::process::Stdio::null())
            .status()
            .context("Failed to extract AppImage")?;

        if !status.success() {
            bail!("extraction failed with {}", status);
        }
    }

    Ok(())
}

fn confirm_overwrite(container: &Container, overwrite_metadata: &DesktopMetadata) -> Result<bool> {
    let metadata = desktop::desktop_metadata(&container.desktop_path())?;

    print!(
        "An app with id '{}' already exists: {}",
        container.id(),
        metadata.name()
    );
    println_version_or(metadata.version(), "");

    print!("Installing: {}", overwrite_metadata.name());
    println_version_or(overwrite_metadata.version(), "");

    prompt("Overwrite? [y/N]: ")
}

fn prompt(prompt: &str) -> Result<bool> {
    loop {
        print!("{}", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" | "" => return Ok(false),
            _ => continue,
        }
    }
}

fn println_version_or(version: Option<&str>, fallback: &str) {
    match version {
        Some(v) => println!(" ({v})."),
        None => println!("{fallback}"),
    }
}

pub fn remove(id: &str, force: bool) -> Result<()> {
    let slug = desktop::slug(id);
    let data_dir = paths::appto_data()?;
    let container = Container::new(&data_dir, &slug);
    if !container.root().is_dir() {
        bail!("No app with id '{slug}'");
    }

    if !force && !prompt(&format!("Removing '{slug}', is this okay? [y/N]: "))? {
        println!("Aborted.");
        return Ok(());
    }

    let application_dir = paths::application_dir()?;
    let bin_dir = paths::bin_dir()?;
    if let Err(e) = container.remove_symlink_desktop(&application_dir) {
        eprintln!("Warning: {e:#}");
    }
    if let Err(e) = container.remove_symlink_appimage(&bin_dir) {
        eprintln!("Warning: {e:#}");
    }

    fs::remove_dir_all(container.root())?;
    println!("Successfully removed '{slug}'");
    Ok(())
}

pub fn list() -> Result<()> {
    let data_dir = paths::appto_data()?;
    // Vec<Container> for each directories. For each, get desktop metadata.
    // Format: - id (name) version

    let containers = container::containers_from(&data_dir)?;
    println!("Installed AppImages: (ID, NAME, VERSION).");
    // Checks if desktop entry is valid, if not print (broken) for name
    for con in containers {
        match desktop::desktop_metadata(&con.desktop_path()) {
            Ok(m) => {
                print!("- {} ({}", con.id(), m.name());
                match m.version() {
                    Some(v) => println!(" v{})", v),
                    None => println!(")"),
                }
            }
            Err(_) => println!("- {} (broken)", con.id()),
        }
    }
    Ok(())
}

pub fn sync(id: &Option<String>) -> Result<()> {
    let data_dir = paths::appto_data()?;
    let cache_dir = paths::appto_cache()?;
    let squash_dir = cache_dir.join("squashfs-root");

    match id {
        Some(id) => sync_one(id, &data_dir, &cache_dir, &squash_dir)?,
        None => sync_all(&data_dir, &cache_dir, &squash_dir)?,
    }
    Ok(())
}

fn sync_one(id: &str, data_dir: &Path, cache_dir: &Path, squash_dir: &Path) -> Result<()> {
    let slug = desktop::slug(id);
    let container = Container::new(data_dir, &slug);
    if !container.root().is_dir() {
        bail!("{} is not installed", container.id());
    }

    if sync_container(&container, cache_dir, squash_dir)? {
        println!("Synced {}", container.id());
    } else {
        println!("{} is already up to date.", container.id());
    }
    Ok(())
}

fn sync_all(data_dir: &Path, cache_dir: &Path, squash_dir: &Path) -> Result<()> {
    let containers = container::containers_from(data_dir)?;

    let mut synced = 0;
    for con in containers {
        if !sync_container(&con, cache_dir, squash_dir)? {
            continue;
        }

        println!("Synced {}", con.id());
        synced += 1;
    }

    if synced == 0 {
        println!("Everything up to date.");
    } else {
        println!("Synced {synced} container(s).");
    }
    Ok(())
}

pub fn sync_container(container: &Container, cache_dir: &Path, squash_dir: &Path) -> Result<bool> {
    extract_appimage(&container.appimage_path(), cache_dir, squash_dir)?;
    let appimage_desktop = desktop::desktop_from(squash_dir)?;
    let installed_desktop = container.desktop_path();

    let appimage_contents =
        fs::read_to_string(appimage_desktop).context("Failed to read squash .desktop file")?;
    let appimage_contents = desktop::updated_desktop(&appimage_contents, container)?;
    let installed_contents =
        fs::read_to_string(&installed_desktop).context("Failed to read container .desktop file")?;

    if let Err(e) = fs::remove_dir_all(squash_dir) {
        eprintln!("Could not remove squashfs-root temp file: {e:#}");
    }
    if appimage_contents == installed_contents {
        return Ok(false); // up to date, nothing written
    }
    container.install_desktop(&appimage_contents)?;
    Ok(true)
}
