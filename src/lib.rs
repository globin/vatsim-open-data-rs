mod airport;
mod position;
mod sector;
mod volume;

use serde::Serialize;
use std::{collections::HashMap, io, path::Path};
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
    InvalidVolume(String, String, volume::ConstraintError),
}

#[derive(Serialize)]
pub struct FIR {
    pub airports: HashMap<String, Airport>,
    pub positions: HashMap<String, Position>,
    pub sectors: HashMap<String, Sector>,
    pub volumes: HashMap<String, Volume>,
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

#[derive(Serialize)]
pub struct OpenData {
    pub firs: HashMap<String, FIR>,
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
            .collect::<Vec<_>>();
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }
}

#[cfg(test)]
mod tests {}
