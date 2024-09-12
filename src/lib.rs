mod airport;
mod position;
mod sector;
pub mod vateud8;
mod volume;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::read_to_string, io, path::Path};
use thiserror::Error;
use tracing::{info, warn};

pub use airport::Airport;
pub use position::Position;
pub use sector::Sector;
pub use volume::Volume;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read file: {0}")]
    FileRead(#[from] io::Error),
    #[error("failed to deserialize toml file: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("Invalid volumes: {0}")]
    ParseVolume(#[from] volume::ReadError),
    #[error("Invalid volumes: {0}, {1}, {2}")]
    InvalidVolume(FirName, VolumeId, volume::ConstraintError),
    #[error("Duplicate Positions: {0}-{1}, {2}-{3}")]
    DuplicatePosition(FirName, PositionId, FirName, PositionId),
}

type FirName = String;
type PositionId = String;
type VolumeId = String;

#[derive(Default, Serialize)]
pub struct FIR {
    pub airports: HashMap<String, Airport>,
    pub positions: HashMap<PositionId, Position>,
    pub sectors: HashMap<String, Sector>,
    pub volumes: HashMap<VolumeId, Volume>,
}

impl FIR {
    // TODO propagate errors? not found files ok/allowlist,
    fn from_folder(path: &Path) -> Self {
        let positions = Position::from_toml(&path.join("positions.toml")).unwrap_or_else(|e| {
            info!(
                "Could not receive position data from {}: {e}",
                path.display()
            );
            HashMap::default()
        });
        let sectors = Sector::from_toml(&path.join("sectors.toml")).unwrap_or_else(|e| {
            info!("Could not receive sector data from {}: {e}", path.display());
            HashMap::default()
        });
        let volumes = Volume::from_geojson(&path.join("volumes.geojson")).unwrap_or_else(|e| {
            info!("Could not receive volume data from {}: {e}", path.display());
            HashMap::default()
        });
        // TODO airports, volumes
        Self {
            airports: HashMap::new(),
            positions,
            sectors,
            volumes,
        }
    }

    fn run_checks(&self) -> Result<(), Vec<(&String, volume::ConstraintError)>> {
        let errs = self
            .volumes
            .iter()
            .filter_map(|(id, vol)| vol.check_level().map_err(|e| (id, e)).err())
            .collect::<Vec<_>>();
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    vateud8: Vateud8Config,
    firs: HashMap<FirName, FirConfig>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Vateud8Config {
    #[serde(default)]
    ignore_regions: Vec<u32>,
    #[serde(default)]
    ignore_extra: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FirConfig {
    vateud8_region: Option<u32>,
    #[serde(default)]
    vateud8_ignore: Vec<String>,
    #[serde(default)]
    optional_frequency: bool,
}

#[derive(Default, Serialize)]
pub struct OpenData {
    pub firs: HashMap<FirName, FIR>,
    pub config: Config,
}

impl OpenData {
    pub fn from_path(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            firs: path
                .join("FIRs")
                .read_dir()?
                .filter_map(|fir_folder| {
                    match fir_folder.map(|folder| {
                        (
                            folder.file_name().to_string_lossy().to_string(),
                            FIR::from_folder(&folder.path()),
                        )
                    }) {
                        Ok(fir_entry) => Some(fir_entry),
                        Err(e) => {
                            warn!("{e}");
                            None
                        }
                    }
                })
                .collect(),
            config: toml::from_str(&read_to_string(path.join("config.toml"))?)?,
        })
    }

    fn positions(&self) -> impl Iterator<Item = (&FirName, &PositionId, &Position)> {
        self.firs.iter().flat_map(|(fir_name, fir)| {
            fir.positions
                .iter()
                .map(move |(pos_id, pos)| (fir_name, pos_id, pos))
        })
    }

    pub fn run_checks(&self) -> Result<(), Vec<Error>> {
        let errs = self
            .firs
            .iter()
            .filter_map(|(fir_name, fir)| {
                fir.run_checks()
                    .map_err(|errs| {
                        errs.into_iter().map(|(vol, err)| {
                            Error::InvalidVolume(fir_name.clone(), vol.clone(), err)
                        })
                    })
                    .err()
            })
            .flatten()
            .chain(self.position_dupe_check().err().unwrap_or_default())
            .collect::<Vec<_>>();
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }

    fn position_dupe_check(&self) -> Result<(), Vec<Error>> {
        let errors = self
            .positions()
            .sorted_by_key(|e| e.0)
            .flat_map(|(fir, pos_id, pos)| {
                self.positions()
                    .sorted_by_key(|e| e.0)
                    .filter(move |(other_fir, other_pos_id, other_pos)| {
                        (fir != *other_fir || pos_id != *other_pos_id)
                            && pos.prefix.starts_with(&other_pos.prefix)
                            && pos.frequency == other_pos.frequency
                    })
                    .map(|(other_fir, other_pos, _)| {
                        Error::DuplicatePosition(
                            fir.to_string(),
                            pos_id.to_string(),
                            other_fir.to_string(),
                            other_pos.to_string(),
                        )
                    })
            })
            .collect::<Vec<_>>();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{position::StationType, Error, OpenData, Position, FIR};

    #[test]
    fn test_pos_dupe() {
        let open_data = OpenData {
            firs: HashMap::from([
                (
                    "TEST".to_string(),
                    FIR {
                        positions: HashMap::from([(
                            "POS1".to_string(),
                            Position {
                                frequency: 134_150_000,
                                prefix: "EDMM".to_string(),
                                station_type: StationType::Center,
                                radio_callsign: "Test Radar".to_string(),
                                name: None,
                                cpdlc_logon: None,
                                airspace_groups: vec![],
                                gcap_tier: None,
                            },
                        )]),
                        ..Default::default()
                    },
                ),
                (
                    "AAAA".to_string(),
                    FIR {
                        positions: HashMap::from([(
                            "POS2".to_string(),
                            Position {
                                frequency: 134_150_000,
                                prefix: "EDM".to_string(),
                                station_type: StationType::Center,
                                radio_callsign: "Aahh Radar".to_string(),
                                name: None,
                                cpdlc_logon: None,
                                airspace_groups: vec![],
                                gcap_tier: None,
                            },
                        )]),
                        ..Default::default()
                    },
                ),
                (
                    "EDMM".to_string(),
                    FIR {
                        positions: HashMap::from([
                            (
                                "DMSD".to_string(),
                                Position {
                                    frequency: 132_305_000,
                                    prefix: "EDDM".to_string(),
                                    station_type: StationType::Approach,
                                    radio_callsign: "München Director".to_string(),
                                    name: Some("München Director South".to_string()),
                                    cpdlc_logon: None,
                                    airspace_groups: vec![],
                                    gcap_tier: None,
                                },
                            ),
                            (
                                "DMSE".to_string(),
                                Position {
                                    frequency: 132_305_000,
                                    prefix: "ED".to_string(),
                                    station_type: StationType::Approach,
                                    radio_callsign: "München Director".to_string(),
                                    name: Some("München Director South".to_string()),
                                    cpdlc_logon: None,
                                    airspace_groups: vec![],
                                    gcap_tier: None,
                                },
                            ),
                        ]),
                        ..Default::default()
                    },
                ),
            ]),
            ..Default::default()
        };

        let check_res = open_data.position_dupe_check();
        assert!(check_res.is_err());

        let err_vec = check_res.unwrap_err();
        eprintln!("{err_vec:?}");
        assert_eq!(err_vec.len(), 2);

        match &err_vec[0] {
            Error::DuplicatePosition(fir1, pos1, fir2, pos2) => {
                assert_eq!(fir1, "EDMM");
                assert_eq!(pos1, "DMSD");
                assert_eq!(fir2, "EDMM");
                assert_eq!(pos2, "DMSE");
            }
            _ => unreachable!("must be duplicate position"),
        }

        match &err_vec[1] {
            Error::DuplicatePosition(fir1, pos1, fir2, pos2) => {
                assert_eq!(fir1, "TEST");
                assert_eq!(pos1, "POS1");
                assert_eq!(fir2, "AAAA");
                assert_eq!(pos2, "POS2");
            }
            _ => unreachable!("must be duplicate position"),
        }
    }
}
