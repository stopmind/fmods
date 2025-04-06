use crate::factorio_api::FactorioApi;
use crate::mod_info::DependencyType::Require;
use crate::mod_info::{Dependency, DependencyType, ModInfo, Version};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::mem::take;
use crate::instance::Instance;

pub fn is_mod_game_content(id: &str) -> bool {
    id == "base" ||
        id == "quality" ||
        id == "elevated-rails" ||
        id == "space-age"
}

struct ExtendedDependency {
    version: Option<Version>,
    dependency_type: DependencyType,
    usages_count: i64
}

struct DependenciesProcessor<'a> {
    need_process: Vec<Dependency>,
    dependencies: HashMap<String, ExtendedDependency>,
    factorio_api: &'a FactorioApi<'a>,
    instance: &'a Instance,
}

impl<'a> DependenciesProcessor<'a> {
    fn new(factorio_api: &'a FactorioApi<'a>, instance: &'a Instance) -> Self {
        DependenciesProcessor { factorio_api, instance, need_process: vec![], dependencies: HashMap::new() }
    }

    fn process_dependency(&mut self, mut dependency: Dependency) -> Result<(), Error> {
        if self.check_satisfied(&dependency) {
            return Ok(());
        }

        match dependency.dependency_type {
            Require => {
                if is_mod_game_content(dependency.mod_id.as_str()) {
                    self.add_dependency(dependency, None);
                    return Ok(());
                };

                let mod_info = match self.factorio_api.get_mod(&dependency.mod_id) {
                    Ok(mod_info) => mod_info,
                    Err(err) => return Err(Error::ModNotFound(dependency.mod_id, err)),
                };

                let mod_release = match if let Some(version) = &dependency.version {
                    mod_info.releases.iter().find(|x| &x.version == version )
                } else {
                    mod_info.releases.last()
                } {
                    Some(release) => release,
                    None => return Err(Error::CantFoundSuitableRelease(dependency.mod_id))
                };

                self.need_process.append(&mut mod_release.info_json.dependencies.clone());

                dependency.version = Some(mod_release.version.clone());
                self.add_dependency(dependency, Some(mod_info));
            },
            _ => self.add_dependency(dependency, None)
        }

        Ok(())
    }

    fn check_satisfied(&mut self, dependency: &Dependency) -> bool {
        if let Some(installed_mod) = self.instance.mods.iter().find(|x| x.name == dependency.mod_id) {
            if let Some(version) = &dependency.version {
                if &installed_mod.version >= version {
                    return true;
                }
            }
            else {
                return true;
            };
        }

        if let Some(extended_dependency) = self.dependencies.get_mut(&dependency.mod_id) {
            if dependency.dependency_type != Require {
                return true;
            }

            let result = match &dependency.version {
                None => true,
                Some(dependency_version) => {
                    if let Some(version) = &extended_dependency.version {
                        version >= dependency_version
                    } else {
                        false
                    }
                }
            };

            if result {
                extended_dependency.usages_count += 1;
            }

            result
        } else {
            false
        }
    }

    fn add_dependency(&mut self, dependency: Dependency, mod_info: Option<ModInfo>) {
        let mut remove_usages_for = None;

        if let Some(extended_dependency) = self.dependencies.get_mut(&dependency.mod_id) {
            extended_dependency.usages_count += 1;

            if let Some(version) = &extended_dependency.version {
                if let Some(dependency_version) = &dependency.version {
                    if dependency_version > version {
                        if let Some(mod_info) = mod_info {
                            remove_usages_for = match mod_info.releases.iter().find(|release| {&release.version == version}) {
                                None => None,
                                Some(release) => Some(release.info_json.dependencies.clone())
                            }
                        }

                        extended_dependency.version = Some(dependency_version.clone());
                    }
                }
            }

        } else {
            self.dependencies.insert(dependency.mod_id, ExtendedDependency {
                version: dependency.version,
                dependency_type: dependency.dependency_type,
                usages_count: 1
            });
        }

        if let Some(remove_usages_for) = remove_usages_for {
            for dependency in remove_usages_for {
                self.remove_usage(&dependency)
            }
        }
    }

    fn remove_usage(&mut self, dependency: &Dependency) {
        if let Some(extended_dependency) = self.dependencies.get_mut(&dependency.mod_id) {
            extended_dependency.usages_count -= 1;
        } else {
            self.dependencies.insert(dependency.mod_id.clone(), ExtendedDependency {
                version: dependency.version.clone(),
                dependency_type: dependency.dependency_type.clone(),
                usages_count: -1,
            });
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ModNotFound(String, ureq::Error),
    CantFoundSuitableRelease(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ModNotFound(id, err) =>
                write!(f, "The mod \"{}\" not found (reason: {})", id, err),
            Error::CantFoundSuitableRelease(id) =>
                write!(f, "Failed to select release for \"{}\" ", id),
        }
    }
}

impl std::error::Error for Error {}

pub fn process_dependencies<'a>(factorio_api: &'a FactorioApi<'a>, instance: &'a Instance, id: String, version: Version)
                        -> Result<Vec<Dependency>, Error> {
    let mut processor = DependenciesProcessor::new(factorio_api, instance);

    processor.need_process.push(Dependency::new(id, Some(version), Require));

    while processor.need_process.len() != 0 {
        for dependency in take(&mut processor.need_process) {
            processor.process_dependency(dependency)?;
        }
    }

    Ok(processor.dependencies.into_iter()
        .filter(|dependency| dependency.1.usages_count > 0)
        .map(|x| Dependency::new(x.0, x.1.version, x.1.dependency_type))
        .collect())
}

pub struct InstallChange {
    pub id: String,
    pub version: Version,
}

pub struct UpdateChange {
    pub id: String,
    pub old_version: Version,
    pub new_version: Version,
}

pub struct Changes {
    pub install: Vec<InstallChange>,
    pub update: Vec<UpdateChange>,
    pub conflicts: Vec<String>
}

impl Changes {
    pub fn compute(instance: &Instance, dependencies: &Vec<Dependency>) -> Self {
        let mut install: Vec<InstallChange> = Vec::new();
        let mut update: Vec<UpdateChange> = Vec::new();
        let mut conflicts: Vec<String> = Vec::new();

        for dependency in dependencies {
            match dependency.dependency_type {
                DependencyType::Conflict => {
                    if let Some(_) = instance.mods.iter().find(|x| { x.name == dependency.mod_id }) {
                        conflicts.push(dependency.mod_id.clone());
                    }
                }
                Require => {
                    if is_mod_game_content(dependency.mod_id.as_str()) {
                        continue
                    }

                    let version = dependency.version.clone().unwrap();

                    if let Some(installed) = instance.mods.iter().find(|x| x.name == dependency.mod_id) {
                        if version > installed.version {
                            update.push(UpdateChange{id: dependency.mod_id.clone(), old_version: installed.version.clone(), new_version: version});
                        }
                    } else {
                        install.push(InstallChange{ id: dependency.mod_id.clone(), version})
                    }
                }
                DependencyType::Optional => {}
            }
        }

        Changes {
            install,
            update,
            conflicts,
        }
    }
}