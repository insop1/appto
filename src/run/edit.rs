use crate::container::Container;
use crate::overrides;
use crate::paths;

use anyhow::{bail, Context, Result};
use std::process::Command;
use std::path::Path;
use std::fs;

pub fn edit(id: &str) -> Result<()> {
    let data_dir = paths::appto_data()?;
    let container = Container::new(&data_dir, id);

    let desktop = container.desktop_path();
    let desktop_contents = fs::read_to_string(&desktop).context("No desktop file exists in container")?;
    let override_desktop = container.override_path();

    if !&override_desktop.is_file() {
        let override_template = overrides::override_template(&desktop_contents)?;
        container.install_override(&override_template).context("Failed write to override.desktop")?;
    }
    edit_overrides(&override_desktop).context("Failed to write with editor")?;

    let override_content = fs::read_to_string(override_desktop)?;
    let override_map = overrides::parse(&override_content);
    let new_contents = overrides::merge_contents(&desktop_contents, override_map)?;

    container.install_desktop(&new_contents)?;

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
