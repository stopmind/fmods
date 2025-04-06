use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};
use std::fmt::Formatter;
use std::fmt::Display;
use std::num::ParseIntError;
use std::str::FromStr;
use crate::mod_info::DependencyType::{Conflict, Optional, Require};

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Version {
    pub major: i64,
    pub minor: i64,
    pub patch: i64
}

impl Version {
    pub fn new(major: i64, minor: i64, patch: i64) -> Self {
        Version {
            minor, major, patch
        }
    }
}

impl FromStr for Version {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();

        let major = match parts.get(0) {
            None => 0,
            Some(value) => value.parse()?,
        };


        let minor = match parts.get(1) {
            None => 0,
            Some(value) => value.parse()?,
        };

        let patch = match parts.get(2) {
            None => 0,
            Some(value) => value.parse()?,
        };

        Ok(Version {
            major,
            minor,
            patch
        })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.major != other.major {
            return if self.major > other.major { Greater } else { Less }
        }

        if self.minor != other.minor {
            return if self.minor > other.minor { Greater } else { Less }
        }

        if self.patch != other.patch {
            return if self.patch > other.patch { Greater } else { Less }
        }

        Equal
    }
}

struct VersionVisitor;

impl<'de> Visitor<'de> for VersionVisitor {
    type Value = Version;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an string of format \"0.0.0\" ")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error
    {
        match Version::from_str(v) {
            Ok(value) => Ok(value),
            Err(err) => Err(E::custom(err))
        }
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        deserializer.deserialize_str(VersionVisitor)
    }
}

#[derive(Deserialize, Debug)]
pub struct ModInfo {
    pub releases: Vec<ModRelease>
}

#[derive(Deserialize, Debug)]
pub struct ModRelease {
    pub version: Version,
    pub info_json: ModReleaseInfoJson
}

#[derive(Deserialize, Debug)]
pub struct ModReleaseInfoJson {
    pub dependencies: Vec<Dependency>,
    pub factorio_version: Version
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DependencyType {
    Conflict,
    Require,
    Optional
}

impl Display for DependencyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Conflict => "Conflict",
            Require  => "Require",
            Optional => "Optional"
        })
    }
}

#[derive(Clone, Debug)]
pub struct Dependency {
    pub mod_id: String,
    pub version: Option<Version>,
    pub dependency_type: DependencyType
}

struct DependencyVisitor;

impl<'de> Visitor<'de> for DependencyVisitor {
    type Value = Dependency;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a valid dependency string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error
    {
        match Dependency::from_str(v) {
            Ok(dependency) => Ok(dependency),
            Err(err) => Err(E::custom(err))
        }
    }
}

impl<'de> Deserialize<'de> for Dependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        deserializer.deserialize_str(DependencyVisitor)
    }
}

impl Dependency {
    pub fn new(mod_id: String, version: Option<Version>, dependency_type: DependencyType) -> Self {
        Dependency {mod_id, version, dependency_type}
    }
}

impl FromStr for Dependency {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // I will implement better parser for dependencies.... I think
        let mut clear = s.to_string()
            .replace("(", "")
            .replace(")", "")
            .replace("~", "");

        let dependency_type =
            if      clear.starts_with("!") { Conflict }
            else if clear.starts_with("?") { Optional }
            else                                { Require };

        clear = clear.replace("!", "")
            .replace("?", "");

        let mut mod_id;
        let mut version = None;
        let parts: Vec<&str> = clear.split(">=").collect();

        if parts.len() == 2 {
            mod_id = parts[0].to_string();
            version = Some(Version::from_str(parts[1].trim_start().trim_end())?);
        } else {
            mod_id = clear;
        }

        mod_id = mod_id.trim_start().trim_end().to_string();

        Ok(Dependency{
            mod_id,
            version,
            dependency_type
        })
    }
}