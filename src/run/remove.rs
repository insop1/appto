use crate::container::Container;
use crate::paths;

use anyhow::{Result, bail};
use std::fs;

pub fn remove(id: &str, force: bool) -> Result<()> {
    let data_dir = paths::appto_data()?;
    let container = Container::new(&data_dir, id);
    if !container.root().is_dir() {
        bail!("No app with id '{id}'");
    }

    if !force && !super::prompt(&format!("Removing '{id}', is this okay? [y/N]: "))? {
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
    println!("Successfully removed '{id}'");
    Ok(())
}
