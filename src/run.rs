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
    let contents = fs::read_to_string(&desktop).context("Failed to read squash .desktop file")?;
    let metadata = desktop::desktop_metadata(&contents)?;

    // Overwrite check then lock
    let container = Container::new(&data_dir, metadata.slug());
    if !force && container.root().is_dir() && !confirm_overwrite(&container, &metadata)? {
        println!("Aborted.");
        return Ok(());
    }

    container.create()?;
    let new_content = desktop::updated_desktop(&contents, &container)?;
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
    match metadata.version() {
        Some(v) => println!("Successfully installed {} v{v}", metadata.name()),
        None => println!("Successfully installed {}", metadata.name()),
    }
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

    // Dumb extraction pattern. For less complexity and faster than a full extraction
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
    let desktop = container.desktop_path();
    let contents = fs::read_to_string(&desktop).unwrap_or_default();

    match desktop::desktop_metadata(&contents) {
        Ok(meta) => match meta.version() {
            Some(v) => println!(
                "An app with id '{}' already exists: {} v{v}",
                container.id(),
                meta.name()
            ),
            None => println!(
                "An app with id '{}' already exists: {}",
                container.id(),
                meta.name()
            ),
        },
        Err(_) => {
            println!("An app with id '{}' already exists.", container.id());
        }
    }
    match overwrite_metadata.version() {
        Some(v) => println!("Installing: {} v{v}", overwrite_metadata.name()),
        None => println!("Installing: {}", overwrite_metadata.name()),
    }

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

    let id_width = 32;
    let name_width = 24;

    let containers = container::containers_from(&data_dir)?;
    println!("{:<id_width$} {:<name_width$} VERSION", "ID", "NAME");
    // Checks if desktop entry is valid, if not print (broken) for name
    for container in containers {
        let desktop = container.desktop_path();
        let contents = fs::read_to_string(&desktop).unwrap_or_default();
        match desktop::desktop_metadata(&contents) {
            Ok(m) => match m.version() {
                Some(v) => println!(
                    "{:<id_width$} {:<name_width$} {v}",
                    container.id(),
                    truncate(m.name(), name_width)
                ),
                None => println!(
                    "{:<id_width$} {:<name_width$}",
                    container.id(),
                    truncate(m.name(), name_width)
                ),
            },
            Err(_) => println!("{:<id_width$} {:<name_width$}", container.id(), "BROKEN"),
        }
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let max = max.saturating_sub(3);
    if s.chars().count() > max {
        let trim: String = s.chars().take(max).collect();
        return format!("{trim}...");
    }
    String::from(s)
}

pub fn sync(id: &Option<String>, check: bool) -> Result<()> {
    let data_dir = paths::appto_data()?;
    let cache_dir = paths::appto_cache()?;
    let squash_dir = cache_dir.join("squashfs-root");

    if check {
        println!("Check mode: no changes will be made");
    }
    match id {
        Some(id) => sync_one(id, &data_dir, &cache_dir, &squash_dir, check)?,
        None => sync_all(&data_dir, &cache_dir, &squash_dir, check)?,
    }
    Ok(())
}

fn sync_one(
    id: &str,
    data_dir: &Path,
    cache_dir: &Path,
    squash_dir: &Path,
    check: bool,
) -> Result<()> {
    let slug = desktop::slug(id);
    let container = Container::new(data_dir, &slug);
    if !container.root().is_dir() {
        bail!("{} is not installed", container.id());
    }

    if !sync_container(&container, cache_dir, squash_dir, check)? {
        println!("{} is already up to date.", container.id());
    }
    Ok(())
}

fn sync_all(data_dir: &Path, cache_dir: &Path, squash_dir: &Path, check: bool) -> Result<()> {
    let containers = container::containers_from(data_dir)?;

    let mut synced = 0;
    for con in containers {
        if !sync_container(&con, cache_dir, squash_dir, check)? {
            continue;
        }
        synced += 1;
    }

    if synced == 0 {
        println!("Everything up to date.");
    } else if check {
        println!("{synced} container(s) would be synced.");
    } else {
        println!("Synced {synced} container(s).");
    }
    Ok(())
}

pub fn sync_container(
    container: &Container,
    cache_dir: &Path,
    squash_dir: &Path,
    check: bool,
) -> Result<bool> {
    extract_appimage(&container.appimage_path(), cache_dir, squash_dir)?;
    let appimage_desktop = desktop::desktop_from(squash_dir)?;
    let installed_desktop = container.desktop_path();

    let appimage_contents =
        fs::read_to_string(&appimage_desktop).context("Failed to read squash .desktop file")?;
    let appimage_contents = desktop::updated_desktop(&appimage_contents, container)?;
    // If container doesn't have a desktop, just sync anyways
    let installed_contents = fs::read_to_string(&installed_desktop).unwrap_or_default();

    if let Err(e) = fs::remove_dir_all(squash_dir) {
        eprintln!("Could not remove squashfs-root temp file: {e:#}");
    }

    if appimage_contents == installed_contents {
        return Ok(false);
    }
    if !check {
        container.install_desktop(&appimage_contents)?;
    }

    // Gracefully handles instead of panic
    let verb = if check { "Would sync" } else { "Synced" };
    let (appimage_metadata, installed_metadata) = match (
        desktop::desktop_metadata(&appimage_contents),
        desktop::desktop_metadata(&installed_contents),
    ) {
        (Ok(a), Ok(i)) => (a, i),
        _ => {
            eprintln!(
                "Warning: could not parse desktop metadata for {}",
                container.id()
            );
            println!("{verb}: {}", container.id());
            return Ok(true);
        }
    };

    match (installed_metadata.version(), appimage_metadata.version()) {
        (Some(i), Some(a)) => println!(
            "{verb}: {} v{i} -> {} v{a}",
            installed_metadata.name(),
            appimage_metadata.name()
        ),
        (Some(i), None) => println!(
            "{verb}: {} v{i} -> {}",
            installed_metadata.name(),
            appimage_metadata.name()
        ),
        (None, Some(a)) => println!(
            "{verb}: {} -> {} v{a}",
            installed_metadata.name(),
            appimage_metadata.name()
        ),
        _ => println!("{verb}: {}", appimage_metadata.name()),
    }
    Ok(true)
}
