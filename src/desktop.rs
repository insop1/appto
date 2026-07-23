use anyhow::{Context, Result};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use crate::container::Container;

pub struct DesktopMetadata {
    name: String,
    slug: String,
    version: Option<String>,
}

impl DesktopMetadata {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn slug(&self) -> &str {
        &self.slug
    }
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

pub fn updated_desktop(contents: &str, container: &Container) -> Result<String> {
    // What we're looking for is to update exec and icon
    let mut output = String::with_capacity(contents.len());
    for line in contents.lines() {
        if line.strip_prefix("Icon=").is_some() {
            if let Some(ip) = container.icon_path() {
                writeln!(output, "Icon={}", ip.display())?;
            }
        } else if let Some(v) = line.strip_prefix("Exec=") {
            writeln!(
                output,
                "Exec={}",
                rewrite_exec(v, &container.appimage_path())
            )?;
        } else {
            writeln!(output, "{}", line)?;
        }
    }
    Ok(output)
}

pub fn rewrite_exec(value: &str, container_appimage: &Path) -> String {
    let bin = container_appimage.display().to_string();
    let mut replaced = false;

    value
        .split_whitespace()
        .map(|tok| {
            if !replaced && tok != "env" && !tok.contains('=') {
                replaced = true;
                bin.as_str()
            } else {
                tok
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn desktop_metadata(contents: &str) -> Result<DesktopMetadata> {
    let mut name = None;
    let mut version = None;

    for line in contents.lines() {
        if name.is_some() && version.is_some() {
            break;
        }
        if name.is_none()
            && let Some(v) = line.strip_prefix("Name=")
        {
            name = Some(v.trim().to_string());
        } else if version.is_none()
            && let Some(v) = line.strip_prefix("X-AppImage-Version=")
        {
            version = Some(v.trim().to_string());
        }
    }

    let name = name.context("No Name= field in .desktop")?;
    let slug = slug(&name);
    Ok(DesktopMetadata {
        name,
        slug,
        version,
    })
}

pub fn desktop_from(path: &Path) -> Result<PathBuf> {
    fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|ext| ext == "desktop"))
        .context("Could not find .desktop in AppImage")
}

pub fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
