use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::fs::{create_dir, read_dir, remove_dir_all, File};
use std::io;
use std::path::{Path, PathBuf};
use dirs::config_dir;
use serde::Deserialize;
use crate::mod_info::Version;

#[derive(Deserialize)]
pub struct InstalledMod {
    pub version: Version,
    pub name: String
}

pub struct Instance {
    pub path: PathBuf,
    pub version: Version,
    pub game_content_versions: HashMap<String, Version>,
    pub mods: Vec<InstalledMod>,
    pub mods_path: PathBuf
}

#[derive(Debug)]
pub enum Error {
    NotExist,
    BrokenInstance
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Error::NotExist => "The instance directory doesn't exist",
            Error::BrokenInstance => "The instance is broken"
        })
    }
}

impl std::error::Error for Error {}

fn read_mods<P: AsRef<Path>>(path: P) -> io::Result<Vec<InstalledMod>> {
    let mut result = Vec::new();

    for entry in read_dir(path)? {
        let path = entry?.path();

        if !path.is_dir() {
            continue
        }

        if let Ok(file) = File::open(path.join("info.json")) {
            if let Ok(mod_info) = serde_json::from_reader(file) {
                result.push(mod_info);
            }
        }
    }

    Ok(result)
}

impl Instance {
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        if !path.is_dir() {
            return Err(Error::NotExist)
        }

        let game_content_versions = match read_mods(path.join("data")) {
            Ok(mods) => {
                let mut map = HashMap::new();
                for mod_info in mods {
                    map.insert(mod_info.name, mod_info.version);
                }
                map
            },
            Err(_) => {
                return Err(Error::BrokenInstance);
            }
        };

        let version= match game_content_versions.get("base") {
            Some(version) => Version::new(version.major, version.minor, 0),
            None => return Err(Error::BrokenInstance)
        };

        let mods_path = config_dir().unwrap().join("Factorio/mods");
        let mods = match read_mods(&mods_path) {
            Ok(mods) => mods,
            Err(_) => {
                let _ = create_dir(&mods_path);
                vec![]
            }
        };

        Ok(Instance{
            path,
            version,
            game_content_versions,
            mods,
            mods_path
        })
    }

    pub fn remove_mod(&self, mod_name: &str) {
        if let Some(info) = self.mods.iter().find(|x| x.name == mod_name) {
            _ = remove_dir_all(self.mods_path.join(format!("{}_{}", &info.name, &info.version)));
            _ = remove_dir_all(self.mods_path.join(&info.name));
        }
    }
}