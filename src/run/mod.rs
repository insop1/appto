mod add;
mod list;
mod remove;
mod sync;
mod edit;

pub use add::add;
pub use list::list;
pub use remove::remove;
pub use sync::sync;
pub use edit::edit;

use anyhow::{Context, Result, bail};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

pub(super) fn extract_appimage(appimage: &Path, cache_dir: &Path, squash_dir: &Path) -> Result<()> {
    let appimage = appimage
        .canonicalize()
        .context("Failed to resolve AppImage path")?;
    if squash_dir.exists() {
        fs::remove_dir_all(squash_dir).context("Failed to clear squashfs-root")?;
    }

    // We are basically doing chmod +x on the AppImage
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
        "usr/share/applications/*",
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

pub(super) fn prompt(prompt: &str) -> Result<bool> {
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

pub(super) fn truncate(s: &str, max: usize) -> String {
    let max = max.saturating_sub(3);
    if s.chars().count() > max {
        let trim: String = s.chars().take(max).collect();
        return format!("{trim}...");
    }
    String::from(s)
}
