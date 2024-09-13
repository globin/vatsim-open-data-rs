use std::{collections::HashMap, fs::read_to_string, path::Path};

use geo::Point;
use serde::{Deserialize, Serialize};

use crate::position::PositionReference;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Airport {
    pub name: String,
    pub iata_designator: Option<String>,
    pub location: Point,
    pub elevation: Option<i32>,
    pub position_priority: Vec<Vec<PositionReference>>,
    #[serde(default)]
    pub runways: Vec<String>,
}

impl Airport {
    pub fn from_toml(path: &Path) -> Result<HashMap<String, Self>, super::Error> {
        Ok(toml::from_str(&read_to_string(path)?)?)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunwayReference {
    pub icao: String,
    pub designator: String,
}
