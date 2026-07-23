use crate::container::{self};
use crate::desktop::{self};
use crate::paths;

use anyhow::Result;
use std::fs;

pub fn list() -> Result<()> {
    let data_dir = paths::appto_data()?;
    // Vec<Container> for each directories. For each, get desktop metadata.

    let id_width = 32;
    let name_width = 24;

    let containers = container::containers_from(&data_dir)?;
    println!("{:<id_width$} {:<name_width$} VERSION", "ID", "NAME");
    // Checks if desktop entry is valid, if not print (broken) for name
    for container in containers {
        let desktop = container.desktop_path();
        let contents = fs::read_to_string(&desktop).unwrap_or_default();
        match desktop::desktop_metadata(&contents) {
            Ok(m) => match m.version() {
                Some(v) => println!(
                    "{:<id_width$} {:<name_width$} {v}",
                    container.id(),
                    super::truncate(m.name(), name_width)
                ),
                None => println!(
                    "{:<id_width$} {:<name_width$}",
                    container.id(),
                    super::truncate(m.name(), name_width)
                ),
            },
            Err(_) => println!("{:<id_width$} {:<name_width$}", container.id(), "BROKEN"),
        }
    }
    Ok(())
}
