use crate::instance::Instance;
use crate::mod_info::Version;
use std::error::Error;
use std::io::Cursor;
use std::path::PathBuf;
use zip::ZipArchive;

pub struct Downloader {
    path: PathBuf
}

impl Downloader {
    pub fn new(instance: &Instance) -> Self {
        Downloader {
            path: instance.mods_path.clone(),
        }
    }

    pub fn download(&self, id: String, version: Version) -> Result<(), Box<dyn Error>> {
        let mut response = ureq::get(format!("https://mods-storage.re146.dev/{}/{}.zip", id, version))
            .call()?;

        // disable read_to_vec size limit
        let bytes = response.body_mut().with_config().read_to_vec()?;

        let mut archive = ZipArchive::new(Cursor::new(bytes))?;
        archive.extract(&self.path)?;

        Ok(())
    }
}