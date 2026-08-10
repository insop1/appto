use anyhow::Result;
use std::collections::HashMap;
use std::fmt::Write;

// Expected format for override.desktop

// [Desktop Entry]
// # Uncomment the ones you'd like to override, unlisted keys are ignored.
// # These edits will persist when you sync.
//
// # Name=
// # Comment=
// # Categories=
// # Keywords=
// # Icon=
// # Path=
// # Exec=
// # Terminal=
// # StartupWMClass=
// # MimeType=

const KEYS: &[&str] = &[
    "Name",
    "Comment",
    "Categories",
    "Keywords",
    "Icon",
    "Path",
    "Exec",
    "Terminal",
    "StartupWMClass",
    "MimeType",
];

pub fn parse(contents: &str) -> HashMap<String, String> {
    let mut hash_map = HashMap::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if !KEYS.contains(&key) {
            continue;
        }

        hash_map.insert(String::from(key), String::from(value.trim()));
    }

    hash_map
}

// existing_map is the override changes the user makes
// This is to keep user changes across regenerations as edit() regenerates it
pub fn make_template(
    override_map: &HashMap<String, String>,
    existing_overrides: &HashMap<String, String>,
) -> Result<String> {
    let mut override_contents = String::from(concat!(
        "[Desktop Entry]\n",
        "# Uncomment the ones you'd like to override, unlisted keys are ignored.\n",
        "# Comment to undo an override.\n",
        "# These edits will persist when you sync.\n",
        "\n",
    ));
    for &key in KEYS {
        if let Some(value) = existing_overrides.get(key) {
            writeln!(override_contents, "{key}={value}")?;
            continue;
        }
        let value = override_map
            .get(key)
            .map(String::as_str)
            .unwrap_or_default();
        writeln!(override_contents, "# {key}={value}")?;
    }

    Ok(override_contents)
}

pub fn merge_contents(
    desktop_contents: &str,
    mut override_map: HashMap<String, String>,
) -> Result<String> {
    let mut new_contents = String::with_capacity(desktop_contents.len());

    // entry_offset is where we'll append the keys that don't exist in the original
    // Which will be under [Desktop Entry]
    let mut entry_offset: usize = 0;
    let mut is_entry = false;
    for line in desktop_contents.lines() {
        if line.starts_with('[') {
            is_entry = line.trim() == "[Desktop Entry]";
            writeln!(new_contents, "{line}")?;

            if is_entry {
                entry_offset = new_contents.len();
            }
            continue;
        }
        if !is_entry {
            writeln!(new_contents, "{line}")?;
            continue;
        }

        let Some((key, _)) = line.split_once('=') else {
            writeln!(new_contents, "{line}")?;
            continue;
        };
        let key = key.trim();
        let Some(new_value) = override_map.remove(key) else {
            writeln!(new_contents, "{line}")?;
            continue;
        };

        writeln!(new_contents, "{key}={new_value}")?;
    }

    let mut leftover = String::new();
    for &key in KEYS {
        if let Some(value) = override_map.remove(key) {
            writeln!(leftover, "{key}={value}")?;
        }
    }
    new_contents.insert_str(entry_offset, &leftover);

    Ok(new_contents)
}
