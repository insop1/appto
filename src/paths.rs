use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;

fn xdg_or(var: &str, fallback: &str) -> Result<PathBuf> {
    if let Ok(v) = std::env::var(var) {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }

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
