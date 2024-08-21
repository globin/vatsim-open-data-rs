use geo::Point;
use serde::{Deserialize, Serialize};

use crate::position::PositionReference;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Airport {
    pub name: String,
    pub iata_designator: String,
    pub location: Point,
    pub elevation: i32,
    pub position_priority: Vec<Vec<PositionReference>>,
    pub runways: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunwayReference {
    pub icao: String,
    pub designator: String,
}
