use crate::container::Container;
use crate::desktop::{self, DesktopMetadata};
use crate::paths;

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn add(appimage: &Path, force: bool) -> Result<()> {
    let cache_dir = paths::appto_cache()?;
    let data_dir = paths::appto_data()?;
    let squash_dir = cache_dir.join("squashfs-root");
    paths::ensure_paths(&[&cache_dir, &data_dir])?;

    super::extract_appimage(appimage, &cache_dir, &squash_dir)?;

    // By now we should have squashfs-root set from extract_appimage
    // which always extract to squashfs-root in tmp path
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

    // Installing icon has to go before updated_desktop
    // updated_desktop relies on container.icon_path(), which can be None if no icon exists in container

    // .DirIcon is a symlink in AppImages, we need the icon filename itself for other extension support
    let dir_icon = squash_dir.join(".DirIcon");
    let resolved_icon = dir_icon.canonicalize().ok();
    if let Some(icon) = resolved_icon
        && let Err(e) = container.install_icon(&icon)
    {
        eprintln!("Warning: could not install icon: {e:#}");
    }

    let new_content = desktop::updated_desktop(&contents, &container)?;
    container.install_desktop(&new_content)?;
    container.install_appimage(appimage)?;

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

    super::prompt("Overwrite? [y/N]: ")
}
