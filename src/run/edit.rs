use crate::container::Container;
use crate::overrides;
use crate::paths;

use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn edit(id: &str, reset: bool) -> Result<()> {
    let data_dir = paths::appto_data()?;
    let container = Container::new(&data_dir, id);
    if !container.root().is_dir() {
        bail!("{id} does not exist")
    }

    if reset {
        println!("Successfully reset edits from {}", id);
        return reset_edits(&container);
    }

    let desktop = container.desktop_path();
    let original_desktop = container.original_path();
    let original_contents = if original_desktop.is_file() {
        fs::read_to_string(&original_desktop).context("Could not read from .original.desktop")?
    } else {
        let desktop_contents =
            fs::read_to_string(&desktop).context("No desktop file exists in container")?;
        container.install_original(&desktop_contents)?;
        desktop_contents
    };

    let override_desktop = container.override_path();
    let existing_overrides = if override_desktop.is_file() {
        let override_contents = fs::read_to_string(&override_desktop)
            .context("Could not read from .override.desktop")?;
        overrides::parse(&override_contents)
    } else {
        HashMap::new()
    };

    let original_map = overrides::parse(&original_contents);
    let override_template = overrides::make_template(&original_map, &existing_overrides)?;
    container
        .install_override(&override_template)
        .context("Failed write to .override.desktop")?;

    edit_overrides(&override_desktop).context("Failed to write with editor")?;

    // We merge the override_map with the original content because it holds the untouched values
    // The main desktop that's symlinked to applications needs its values overriden to show
    let override_content =
        fs::read_to_string(&override_desktop).context("Failed read from .override.desktop")?;
    let override_map = overrides::parse(&override_content);
    let new_contents = overrides::merge_contents(&original_contents, override_map)?;

    container.install_desktop(&new_contents)?;
    println!("Succesfully edited {id}.");

    Ok(())
}

fn edit_overrides(override_desktop: &Path) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .context("Neither $EDITOR nor $VISUAL are set")?;

    let mut parts = editor.split_whitespace();
    let cmd = parts.next().context("$EDITOR/$VISUAL is empty")?;
    let status = Command::new(cmd)
        .args(parts)
        .arg(override_desktop)
        .status()
        .with_context(|| format!("Failed to launch editor '{editor}'"))?;

    if !status.success() {
        bail!("Editor exited with {}", status);
    }
    Ok(())
}

fn reset_edits(container: &Container) -> Result<()> {
    let desktop = container.desktop_path();
    let override_desktop = container.override_path();
    let original_desktop = container.original_path();

    if !original_desktop.is_file() && !override_desktop.is_file() {
        println!("{} has no edits.", container.id());
        return Ok(());
    }

    if override_desktop.is_file() {
        fs::remove_file(&override_desktop).context("Could not remove .override.desktop")?;
    }
    if !original_desktop.is_file() {
        println!("{}.original.desktop does not exist.", container.id());
        println!("Run 'appto sync' to restore original.");
        return Ok(());
    }

    fs::rename(original_desktop, desktop).context("Could not rename .original.desktop to .desktop")
}
