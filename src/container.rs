use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{bail, Result, Context};
use std::os::unix::fs::symlink;

pub struct Container {
    root: PathBuf,
    id: String,
}

impl Container {
    pub fn new(data_dir: &Path, id: &str) -> Self { 
        Container { 
            root: data_dir.join(id),
            id: String::from(id),
        }
    }

    pub fn appimage_path(&self) -> PathBuf {
        self.root.join(format!("{}.AppImage", self.id))
    }
    pub fn icon_path(&self) -> PathBuf { 
        self.root.join(format!("{}.png", self.id))
    }
    pub fn desktop_path(&self) -> PathBuf {
        self.root.join(format!("{}.desktop", self.id))
    }
    #[allow(dead_code)]
    pub fn root(&self) -> &Path { &self.root }
    pub fn id(&self) -> &str { &self.id }

    pub fn create(&self) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        Ok(())
    }
    pub fn install_desktop(&self, contents: &str) -> Result<()> {
        fs::write(self.desktop_path(), contents).context("Failed to write .desktop file")
    }
    pub fn install_icon(&self, dir_icon: &Path) -> Result<()> {
        fs::copy(dir_icon, self.icon_path()).context("Failed to install icon")?;
        Ok(())
    }
    pub fn install_appimage(&self, appimage: &Path) -> Result<()> {
        fs::copy(appimage, self.appimage_path()).context("Failed to install AppImage")?;
        Ok(())
    }
    pub fn symlink_desktop(&self, application_dir: &Path) -> Result<()> {
        let link = application_dir.join(format!("{}.desktop", &self.id));
        clear_link(&link)?;
        symlink(self.desktop_path(), &link).context("Failed to create symlink for .desktop")
    }
    pub fn symlink_appimage(&self, bin_dir: &Path) -> Result<()> {
        let link = bin_dir.join(&self.id);
        clear_link(&link)?;
        symlink(self.appimage_path(), &link).context("Failed to create symlink for bin")
    }
}

fn clear_link(link: &Path) -> Result<()> {
    match fs::symlink_metadata(link) {
        Ok(meta) if meta.is_symlink() => {
            fs::remove_file(&link).context("Failed to remove old symlink")?;
        }
        Ok(_) => bail!("'{}' exists and is not a symlink; refusing to replace", link.display()),
        Err(_) => {}
    }
    Ok(())
}
