use crate::source::Source;
use crate::HudName;
use chrono::{DateTime, Utc};
use enum_as_inner::EnumAsInner;
use std::collections::BTreeMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Registry {
    info: BTreeMap<HudName, HudInfo>,
}

impl Registry {
    pub fn new() -> Self {
        Registry::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = &HudInfo> {
        self.info.values()
    }

    pub fn add(&mut self, name: HudName, source: Source) {
        self.info.insert(
            name.clone(),
            HudInfo {
                name,
                source,
                install: Install::None,
            },
        );
    }

    pub fn remove(&mut self, name: &HudName) -> Option<HudInfo> {
        self.info.remove(name)
    }

    pub fn get(&self, name: &HudName) -> Option<&HudInfo> {
        self.info.get(name)
    }

    pub fn set_install(&mut self, name: &HudName, install: Install) {
        if let Some(info) = self.info.get_mut(name) {
            info.install = install;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HudInfo {
    pub name: HudName,
    pub source: Source,
    pub install: Install,
}

#[derive(Clone, Debug, EnumAsInner, Serialize, Deserialize)]
pub enum Install {
    None,
    Installed { path: PathBuf, when: DateTime<Utc> },
    Failed { error: String },
}
