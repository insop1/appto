use anyhow::{Context, Result, bail};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

pub struct Container {
    root: PathBuf,
    id: String,
    icon_ext: Option<String>,
}

impl Container {
    pub fn new(data_dir: &Path, id: &str) -> Self {
        Container {
            root: data_dir.join(id),
            id: String::from(id),
            icon_ext: None,
        }
    }

    // Getters
    pub fn appimage_path(&self) -> PathBuf {
        self.root.join(format!("{}.AppImage", self.id))
    }
    pub fn icon_path(&self) -> Option<PathBuf> {
        // If icon_ext is Some, then it means a new icon is installed. So using dumb path is okay here.
        // Otherwise we check if there is an icon in the container and use that path instead
        if let Some(ext) = &self.icon_ext {
            return Some(self.root.join(format!("{}.{}", self.id, ext)));
        }
        ["png", "svg"]
            .iter()
            .map(|ext| self.root.join(format!("{}.{}", self.id, ext)))
            .find(|p| p.exists())
    }
    pub fn desktop_path(&self) -> PathBuf {
        self.root.join(format!("{}.desktop", self.id))
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn desktop_link(&self, application_dir: &Path) -> PathBuf {
        application_dir.join(format!("{}.desktop", self.id))
    }
    pub fn bin_link(&self, bin_dir: &Path) -> PathBuf {
        bin_dir.join(&self.id)
    }

    // Installs
    pub fn create(&self) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        Ok(())
    }
    pub fn install_desktop(&self, contents: &str) -> Result<()> {
        atomic_write(contents, &self.desktop_path())
    }
    pub fn install_icon(&mut self, icon: &Path) -> Result<()> {
        let ext = icon.extension().and_then(|s| s.to_str()).unwrap_or("png");
        let dest = self.root.join(format!("{}.{}", self.id, ext));
        atomic_copy(icon, &dest)?;
        for other in ["png", "svg"] {
            if other != ext {
                let _ = fs::remove_file(self.root.join(format!("{}.{}", self.id, other)));
            }
        }
        self.icon_ext = Some(String::from(ext));
        Ok(())
    }
    pub fn install_appimage(&self, appimage: &Path) -> Result<()> {
        atomic_copy(appimage, &self.appimage_path())
    }

    // Symlinks
    pub fn symlink_desktop(&self, application_dir: &Path) -> Result<()> {
        let link = self.desktop_link(application_dir);
        clear_symlink(&link)?;
        symlink(self.desktop_path(), &link).context("Failed to create symlink for .desktop")
    }
    pub fn symlink_appimage(&self, bin_dir: &Path) -> Result<()> {
        let link = self.bin_link(bin_dir);
        clear_symlink(&link)?;
        symlink(self.appimage_path(), &link).context("Failed to create symlink for bin")
    }
    pub fn remove_symlink_desktop(&self, application_dir: &Path) -> Result<()> {
        let link = self.desktop_link(application_dir);
        remove_symlink(&link, &self.desktop_path())
    }
    pub fn remove_symlink_appimage(&self, bin_dir: &Path) -> Result<()> {
        let link = self.bin_link(bin_dir);
        remove_symlink(&link, &self.appimage_path())
    }
}

fn atomic_copy(from: &Path, to: &Path) -> Result<()> {
    let tmp = format!("{}.tmp", to.display());
    fs::copy(from, &tmp).with_context(|| format!("Failed to copy to {tmp}"))?;
    fs::rename(&tmp, to).context("Failed to atomic swap")?;
    Ok(())
}

fn atomic_write(contents: &str, to: &Path) -> Result<()> {
    let tmp = format!("{}.tmp", to.display());
    fs::write(&tmp, contents).with_context(|| format!("Failed to write to {tmp}"))?;
    fs::rename(&tmp, to).context("Failed to atomic swap")?;
    Ok(())
}

fn remove_symlink(link: &Path, expected_target: &Path) -> Result<()> {
    let target = match fs::read_link(link) {
        Ok(t) => t,
        Err(_) => return Ok(()),
    };

    if target != expected_target {
        bail!(
            "'{}' does not point to '{}', aborting.",
            target.display(),
            expected_target.display()
        );
    }
    fs::remove_file(link).context("Failed to remove symlink")
}

fn clear_symlink(link: &Path) -> Result<()> {
    match fs::symlink_metadata(link) {
        Ok(meta) if meta.is_symlink() => {
            fs::remove_file(link).context("Failed to remove old symlink")?;
        }
        Ok(_) => bail!(
            "'{}' exists and is not a symlink, refusing to clear",
            link.display()
        ),
        Err(_) => {}
    }
    Ok(())
}

pub fn containers_from(path: &Path) -> Result<Vec<Container>> {
    Ok(fs::read_dir(path)
        .context("Failed to read appto data dir")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .map(|id| Container::new(path, &id))
        .collect())
}
