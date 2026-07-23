use crate::container::{self, Container};
use crate::desktop;
use crate::paths;

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

struct SyncArgs {
    check: bool,
    sync_icon: bool,
}

pub fn sync(id: &Option<String>, check: bool, sync_icon: bool) -> Result<()> {
    let data_dir = paths::appto_data()?;
    let cache_dir = paths::appto_cache()?;
    let squash_dir = cache_dir.join("squashfs-root");
    let args = SyncArgs { check, sync_icon };

    if check {
        println!("Check mode: no changes will be made");
    }
    match id {
        Some(id) => sync_one(id, &data_dir, &cache_dir, &squash_dir, &args)?,
        None => sync_all(&data_dir, &cache_dir, &squash_dir, &args)?,
    }
    Ok(())
}

fn sync_one(
    id: &str,
    data_dir: &Path,
    cache_dir: &Path,
    squash_dir: &Path,
    args: &SyncArgs,
) -> Result<()> {
    let slug = desktop::slug(id);
    let mut container = Container::new(data_dir, &slug);
    if !container.root().is_dir() {
        bail!("{} is not installed", container.id());
    }

    if !sync_container(&mut container, cache_dir, squash_dir, args)? {
        println!("{} is already up to date.", container.id());
    }
    Ok(())
}

fn sync_all(data_dir: &Path, cache_dir: &Path, squash_dir: &Path, args: &SyncArgs) -> Result<()> {
    let containers = container::containers_from(data_dir)?;

    let mut synced = 0;
    for mut con in containers {
        match sync_container(&mut con, cache_dir, squash_dir, args) {
            Ok(synced_container) => {
                if !synced_container {
                    continue;
                }
            }
            Err(e) => {
                eprintln!("Warning: Could not sync {}: {e:#}", con.id());
                continue;
            }
        }
        synced += 1;
    }

    if synced == 0 {
        println!("Everything up to date.");
    } else if args.check {
        println!("{synced} container(s) would be synced.");
    } else {
        println!("Synced {synced} container(s).");
    }
    Ok(())
}

fn sync_icon(squash_dir: &Path, container: &mut Container, args: &SyncArgs) -> Result<bool> {
    if !args.sync_icon {
        return Ok(false);
    }

    let dir_icon = squash_dir.join(".DirIcon");
    let Some(new_icon) = dir_icon.canonicalize().ok() else {
        return Ok(false);
    };

    // If no icon found in container, then we'll just install a new one
    let install = match container.icon_path() {
        Some(installed_icon) => fs::read(&new_icon)? != fs::read(&installed_icon)?,
        None => true,
    };
    if !install {
        return Ok(false);
    }
    if !args.check {
        container.install_icon(&new_icon)?;
    }

    let file_name = new_icon
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("icon.png");
    if args.check {
        println!("Would sync: {}", file_name);
    } else {
        println!("Synced: {}", file_name);
    }
    Ok(true)
}

fn sync_container(
    container: &mut Container,
    cache_dir: &Path,
    squash_dir: &Path,
    args: &SyncArgs,
) -> Result<bool> {
    super::extract_appimage(&container.appimage_path(), cache_dir, squash_dir)?;

    // Icon changes need to be before desktop. Check add()
    let icon_synced = match sync_icon(squash_dir, container, args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Warning: Could not sync icon: {e:#}");
            false
        }
    };

    let appimage_desktop = desktop::desktop_from(squash_dir)?;
    let appimage_contents =
        fs::read_to_string(&appimage_desktop).context("Failed to read squash .desktop file")?;
    let appimage_contents = desktop::updated_desktop(&appimage_contents, container)?;

    // If container doesn't have a desktop, just sync anyways
    let installed_desktop = container.desktop_path();
    let installed_contents = fs::read_to_string(&installed_desktop).unwrap_or_default();

    if let Err(e) = fs::remove_dir_all(squash_dir) {
        eprintln!("Warning: could not remove squashfs-root temp file: {e:#}");
    }

    if appimage_contents == installed_contents && !icon_synced {
        return Ok(false);
    }
    if !args.check {
        container.install_desktop(&appimage_contents)?;
    }

    // Message
    // Gracefully handles instead of panic
    let verb = if args.check { "Would sync" } else { "Synced" };
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
