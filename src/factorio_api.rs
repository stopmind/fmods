use crate::instance::Instance;
use crate::mod_info::DependencyType::Require;
use crate::mod_info::{ModInfo, ModRelease};
use crate::utils::is_mod_game_content;
use std::mem::take;
use url::Url;

pub struct FactorioApi<'a> {
    instance: &'a Instance
}

impl<'a> FactorioApi<'a> {
    pub fn new(instance: &'a Instance) -> Self {
        FactorioApi {
            instance
        }
    }

    pub fn get_mod(&self, name: &String) -> Result<ModInfo, ureq::Error> {

        let mut url = format!("https://mods.factorio.com/api/mods/{}/full", name);
        url = match Url::parse(url.as_str()) {
            Ok(url) => url,
            Err(err) => return Err(ureq::Error::Other(err.into()))
        }.to_string();

        let mut response = ureq::get(url)
            .call()?;

        let mut result: ModInfo = response.body_mut().read_json()?;

        result.releases = take(&mut result.releases).into_iter()
            .filter(|x| self.is_release_compatible(x))
            .collect();

        result.releases.sort_by(|x1, x2| x1.version.cmp(&x2.version));

        Ok(result)
    }

    fn is_release_compatible(&self, mod_release: &ModRelease) -> bool {
        if mod_release.info_json.factorio_version != self.instance.version {
            return false;
        }

        for dependency in mod_release.info_json.dependencies.iter() {
            if dependency.dependency_type != Require || !is_mod_game_content(dependency.mod_id.as_str()) {
                continue
            }

            if let Some(version) = self.instance.game_content_versions.get(&dependency.mod_id) {
                if let Some(required_version) = &dependency.version {
                    if required_version > version {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        true
    }
}
