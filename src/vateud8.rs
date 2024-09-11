use chrono::NaiveDate;
use itertools::Itertools;
use reqwest::blocking;
use scraper::{Html, Selector};
use serde::Serialize;
use thiserror::Error;
use tracing::debug;

use crate::OpenData;

const VATEUD8_URL: &str = "https://fsmine.dhis.org/vateud8/";

#[derive(Serialize)]
pub struct Vateud8Data {
    positions: Vec<Vateud8Position>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not fetch vateud8 data: {0}")]
    Fetch(#[from] reqwest::Error),
    #[error("VATEUD8 list, region mismatch: {0}-{1}, {2}!={3}")]
    RegionMismatch(String, String, u32, u32),
    #[error("VATEUD8 list, not found: {0}-{1}, wrong frequency?")]
    NotFound(String, String),
    #[error("VATEUD8 list, extra position: {0}")]
    Superfluous(String),
}

fn fetch_html(url: Option<&str>) -> Result<String, Error> {
    Ok(blocking::get(url.unwrap_or(VATEUD8_URL))?.text()?)
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Hash)]
struct Vateud8Position {
    region: u32,
    name: String,
    callsign: String,
    frequency: u32,
    prefix: String,
    updated_at: Option<NaiveDate>,
}

pub fn get(url: Option<&str>) -> Result<Vateud8Data, Error> {
    let body = fetch_html(url)?;
    let doc = Html::parse_document(&body);

    let positions = doc
        .select(&Selector::parse("table").unwrap())
        .nth(1)
        .unwrap()
        .select(&Selector::parse("tr").unwrap())
        .skip(1)
        .map(|tr| {
            let td = Selector::parse("td").unwrap();
            let mut cells = tr.select(&td);
            Vateud8Position {
                region: cells.next().unwrap().text().join("").parse().unwrap(),
                name: cells.next().unwrap().text().join(""),
                callsign: cells.next().unwrap().text().join(""),
                #[allow(
                    clippy::cast_sign_loss,
                    clippy::cast_possible_truncation,
                    reason = "can only be positive and in range"
                )]
                frequency: (cells
                    .next()
                    .unwrap()
                    .text()
                    .join("")
                    .parse::<f64>()
                    .unwrap()
                    * 1_000.0)
                    .round() as u32
                    * 1000,
                prefix: cells.next().unwrap().text().join(""),
                updated_at: {
                    let date_str = cells.next().unwrap().text().join("");
                    NaiveDate::parse_and_remainder(&date_str, "%Y-%m-%d")
                        .ok()
                        .map(|(date, _)| date)
                },
            }
        })
        .collect();
    Ok(Vateud8Data { positions })
}

impl Vateud8Data {
    pub fn check(&self, open_data: &OpenData) -> Result<(), Vec<Error>> {
        let errors = open_data
            .firs
            .iter()
            .filter_map(|(name, fir)| {
                open_data
                    .config
                    .firs
                    .get(name)
                    .and_then(|fir_config| Some(fir_config).zip(fir_config.vateud8_region))
                    .zip(Some((name, fir)))
            })
            .flat_map(|((fir_config, v8_region), (fir_name, fir))| {
                fir.positions
                    .iter()
                    .filter(|(pos_name, _)| !fir_config.vateud8_ignore.contains(pos_name))
                    .filter_map(move |(position_name, position)| {
                        if let Some(v8_pos) = self.positions.iter().find(|vateud8_pos| {
                            let matches = vateud8_pos.frequency == position.frequency
                                && ((!vateud8_pos.prefix.is_empty()
                                    && vateud8_pos.prefix.starts_with(&position.prefix))
                                    || position
                                        .prefix
                                        .starts_with(vateud8_pos.name.split('_').next().unwrap()));
                            debug!("{vateud8_pos:?}-{position:?}: {matches}");
                            matches
                        }) {
                            if v8_pos.region != v8_region {
                                return Some(Error::RegionMismatch(
                                    fir_name.clone(),
                                    position_name.clone(),
                                    v8_pos.region,
                                    v8_region,
                                ));
                            }
                        } else {
                            return Some(Error::NotFound(fir_name.clone(), position_name.clone()));
                        }

                        None
                    })
            })
            .chain(
                self.positions
                    .iter()
                    .filter(|v8_pos| {
                        !v8_pos.name.ends_with("_ATIS")
                            && !open_data
                                .config
                                .vateud8
                                .ignore_regions
                                .contains(&v8_pos.region)
                            && !open_data.config.vateud8.ignore_extra.contains(&v8_pos.name)
                    })
                    .filter_map(|vateud8_pos| {
                        if open_data
                            .config
                            .firs
                            .iter()
                            .filter(|(_, c)| c.vateud8_region == Some(vateud8_pos.region))
                            .filter_map(|(fir_name, _)| open_data.firs.get(fir_name))
                            .flat_map(|fir| &fir.positions)
                            .any(|(_, position)| {
                                vateud8_pos.frequency == position.frequency
                                    && ((!vateud8_pos.prefix.is_empty()
                                        && vateud8_pos.prefix.starts_with(&position.prefix))
                                        || position.prefix.starts_with(
                                            vateud8_pos.name.split('_').next().unwrap(),
                                        ))
                            })
                        {
                            None
                        } else {
                            Some(Error::Superfluous(vateud8_pos.name.clone()))
                        }
                    }),
            )
            .collect::<Vec<_>>();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
