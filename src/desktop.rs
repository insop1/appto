use std::path::{Path};
use std::fs;
use std::process::Command;
use anyhow::{bail, Context, Result};

pub struct DesktopEntry {
    name: String,
    version: Option<String>,
}

pub fn rewrite_desktop(desktop: &Path) {
    todo!()
}

pub fn desktop_metadata(desktop: &Path) -> Result<DesktopEntry> {
    let contents = fs::read_to_string(desktop).context("Failed to read .desktop file")?;
    
    let mut name = None;
    let mut version = None;

    for line in contents.lines() {
        if name.is_some() && version.is_some() {
            break;
        }
        if name.is_none() && let Some(v) = line.strip_prefix("Name=") {
            name = Some(v.trim().to_string());
        }
        if version.is_none() && let Some(v) = line.strip_prefix("X-AppImage-Version=") {
            version = Some(v.trim().to_string());
        }
    }

    Ok(DesktopEntry {
        name: name.context("No Name= field in .desktop")?,
        version
    })
}

pub fn edit_desktop(desktop: &Path) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .context("Neither $EDITOR nor $VISUAL are set")?;

    let mut parts = editor.split_whitespace();
    let cmd = parts.next().context("$EDITOR $VISUAL is empty")?;
    let status = Command::new(cmd)
        .args(parts)
        .arg(desktop)
        .status()
        .with_context(|| format!("Failed to launch editor '{editor}'"))?;

    if !status.success() {
        bail!("Editor exited with {}", status);
    }

    Ok(())
}

