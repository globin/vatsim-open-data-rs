use std::io;
use std::{collections::HashMap, fs::read_to_string, path::Path};

use geo::Polygon;
use geojson::{feature::Id, GeoJson};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Volume {
    // TODO uom?
    /// Lower vertical boundary as flight level
    lower_level: u64,
    /// Upper vertical boundary as flight level
    upper_level: u64,
    /// lateral boundary
    lateral_bounds: Polygon,
}

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("failed to read file: {0}")]
    FileRead(#[from] io::Error),
    #[error("missing lower_level for {0} in {1}")]
    MissingLowerLevel(String, String),
    #[error("invalid lower_level for {0} in {1}")]
    InvalidLowerLevel(String, String),
    #[error("missing upper_level for {0} in {1}")]
    MissingUpperLevel(String, String),
    #[error("invalid upper_level for {0} in {1}")]
    InvalidUpperLevel(String, String),
    #[error("missing geometry for {0} in {1}")]
    MissingGeometry(String, String),
    #[error("invalid id {0} in {1}")]
    InvalidId(String, String),
    #[error("missing id in {0}")]
    MissingId(String),
    #[error("no FeatureCollection in {0}")]
    NoFeatureCollection(String),
    #[error("failed to deserialize geojson file: {0}")]
    GeoJsonDeserialize(#[from] geojson::Error),
}

#[derive(Debug, Error)]
pub enum ConstraintError {
    #[error("lower_level must be lesser than upper_level")]
    LowerLevelGreater,
    #[error("upper_level must be lower than 999")]
    UpperLevelMaximum,
}

impl Volume {
    pub fn from_geojson(path: &Path) -> Result<HashMap<String, Self>, ReadError> {
        let geojson_str = read_to_string(path)?;
        let geojson = geojson_str.parse::<GeoJson>()?;
        if let GeoJson::FeatureCollection(feature_collection) = geojson {
            feature_collection
                .features
                .iter()
                .map(|feature| match feature.id {
                    Some(Id::String(ref id)) => Ok((
                        id.clone(),
                        Self {
                            lateral_bounds: feature
                                .geometry
                                .as_ref()
                                .ok_or(ReadError::MissingGeometry(
                                    id.clone(),
                                    path.display().to_string(),
                                ))?
                                .value
                                .clone()
                                .try_into()?,
                            lower_level: feature
                                .property("lower_level")
                                .ok_or_else(|| {
                                    ReadError::MissingLowerLevel(
                                        id.clone(),
                                        path.display().to_string(),
                                    )
                                })?
                                .as_u64()
                                .ok_or_else(|| {
                                    ReadError::InvalidLowerLevel(
                                        id.clone(),
                                        path.display().to_string(),
                                    )
                                })?,
                            upper_level: feature
                                .property("upper_level")
                                .ok_or_else(|| {
                                    ReadError::MissingUpperLevel(
                                        id.clone(),
                                        path.display().to_string(),
                                    )
                                })?
                                .as_u64()
                                .ok_or_else(|| {
                                    ReadError::InvalidUpperLevel(
                                        id.clone(),
                                        path.display().to_string(),
                                    )
                                })?,
                        },
                    )),
                    Some(Id::Number(ref id)) => Err(ReadError::InvalidId(
                        id.to_string(),
                        path.display().to_string(),
                    )),
                    None => Err(ReadError::MissingId(path.display().to_string())),
                })
                .fold_ok(HashMap::new(), |mut acc, (id, volume)| {
                    acc.insert(id, volume);
                    acc
                })
        } else {
            Err(ReadError::NoFeatureCollection(path.display().to_string()))
        }
    }

    pub fn check_level(&self) -> Result<(), ConstraintError> {
        if self.lower_level >= self.upper_level {
            return Err(ConstraintError::LowerLevelGreater);
        }
        if self.upper_level > 999 {
            return Err(ConstraintError::UpperLevelMaximum);
        }
        Ok(())
    }
}
