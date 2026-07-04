use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

fn xdg_or(var: &str, fallback: &str) -> Result<PathBuf> {
    if let Ok(v) = std::env::var(var)
        && !v.is_empty()
    {
        return Ok(PathBuf::from(v));
    };

    let home = std::env::var("HOME").context("Home not found.")?;
    Ok(PathBuf::from(home).join(fallback))
}

pub fn ensure_paths(paths: &[&Path]) -> Result<()> {
    for path in paths {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn appto_data() -> Result<PathBuf> {
    Ok(xdg_or("XDG_DATA_HOME", ".local/share")?.join("appto"))
}

pub fn appto_cache() -> Result<PathBuf> {
    Ok(xdg_or("XDG_CACHE_HOME", ".cache")?.join("appto"))
}

pub fn application_dir() -> Result<PathBuf> {
    Ok(xdg_or("XDG_DATA_HOME", ".local/share")?.join("applications"))
}

pub fn bin_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("Home not found.")?;
    Ok(PathBuf::from(home).join(".local/bin"))
}

pub fn warn_path_missing(bin_dir: &Path) {
    let Ok(path) = std::env::var("PATH") else {
        return;
    };
    if !std::env::split_paths(&path).any(|p| p == bin_dir) {
        eprintln!(
            "Note: {} is not on your PATH, add it to run apps by name",
            bin_dir.display()
        );
    }
}
