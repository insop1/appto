# appto

**appto** is a minimal CLI tool that integrates AppImages into your desktop. It is not a launcher nor a package manager, it just runs once when you call it. Meant for people who aren't AppImage enthusiasts and just want them discoverable by the system.

## Installation

### Crates.io

```bash
cargo install appto
```

### Building from source

```bash
git clone https://github.com/insop1/appto
cd appto
cargo install --path .
```

## Usage

```bash
# Install and integrate an AppImage
# Aliases: install, i
appto add ~/Downloads/Test_Example.AppImage

# List installed apps
# Alias: ls
appto list

# Re-sync a desktop entry after an AppImage self-updates
appto sync test-example

# Sync all installed apps' desktop entries
appto sync

# Sync desktop with the icon
appto sync --icon
appto sync --icon test-example

# Checks sync diff, no changes
# Alias: --dry-run
appto sync --check

# Remove an AppImage (files, menu entry, and symlinks)
# Alias: rm
appto remove test-example

# Edit an AppImage's .desktop file
appto edit test-example

# Resets edits
appto edit --reset

```

## Where things live

**appto** uses these paths by default, however they respect XDG variables.
- App containers: `~/.local/share/appto/`
- Desktop entries: symlinked into `~/.local/share/applications/`
- Binaries: symlinked into `~/.local/bin/` (No XDG)
- Cache: `~/.cache/appto/` 

## Requirements

Linux only. Works with type-2 AppImages.
