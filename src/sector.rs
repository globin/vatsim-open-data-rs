use std::{collections::HashMap, fs::read_to_string, path::Path};

use serde::{Deserialize, Serialize};

use crate::{airport::RunwayReference, position::PositionReference};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sector {
    // TODO add id inside struct?
    pub name: Option<String>,
    pub volumes: Vec<String>,
    #[serde(default)]
    pub runway_filter: Vec<Vec<RunwayReference>>,
    pub position_priority: Vec<Vec<PositionReference>>,
}

impl Sector {
    pub fn from_toml(path: &Path) -> Result<HashMap<String, Self>, super::Error> {
        Ok(toml::from_str(&read_to_string(path)?)?)
    }
}
