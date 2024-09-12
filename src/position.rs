use std::{collections::HashMap, fs::read_to_string, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum StationType {
    #[serde(rename = "DEL")]
    ClearanceDelivery,
    #[serde(rename = "RMP")]
    Ramp,
    #[serde(rename = "RDO")]
    Radio,
    #[serde(rename = "TMU")]
    TrafficManagement,
    #[serde(rename = "FMP")]
    FlowManagement,
    #[serde(rename = "GND")]
    Ground,
    #[serde(rename = "TWR")]
    Tower,
    #[serde(rename = "APP")]
    Approach,
    #[serde(rename = "DEP")]
    Departure,
    #[serde(rename = "CTR")]
    Center,
    #[serde(rename = "FSS")]
    FlightServiceStation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "group", rename_all = "lowercase")]
pub enum GcapTier {
    // TODO None variant instead of Option<GcapTier>?
    One,
    Two(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    // TODO add id inside struct?
    // TODO uom frequency?
    pub frequency: u32,
    pub prefix: String,
    pub station_type: StationType,
    pub name: Option<String>,
    pub radio_callsign: String,
    pub cpdlc_logon: Option<String>,
    #[serde(default)]
    pub airspace_groups: Vec<String>,
    pub gcap_tier: Option<GcapTier>,
}

impl Position {
    pub fn from_toml(path: &Path) -> Result<HashMap<String, Self>, super::Error> {
        Ok(toml::from_str(&read_to_string(path)?)?)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PositionReference {
    pub fir: Option<String>,
    pub id: String,
}
