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
appto add ~/Downloads/Test_Example.AppImage

# List installed apps
appto list

# Re-sync a desktop entry after an AppImage self-updates
appto sync test-example

# Sync all installed apps
appto sync

# Remove an app (files, menu entry, and symlinks)
appto remove test-example
```

`add`, `remove`, and `list` also answer to `install`/`i`, `rm`, and `ls`.

## Where things live

**appto** uses these paths by default, however they respect XDG variables.
- App containers: `~/.local/share/appto/`
- Desktop entries: symlinked into `~/.local/share/applications/`
- Binaries: symlinked into `~/.local/bin/` (No XDG)
- Cache: `~/.cache/appto/` 

## Requirements

Linux only. Works with type-2 AppImages.
