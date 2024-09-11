use std::{
    env::{self, args_os},
    io,
    path::Path,
};

use tracing::error;
use tracing_subscriber::EnvFilter;
use vatsim_open_data::{
    vateud8::{self},
    OpenData,
};

fn main() -> Result<(), vatsim_open_data::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("VATSIM_OPEN_DATA_LOG"))
        .with_writer(io::stderr)
        .init();

    // FIXME clap
    let open_data = OpenData::from_path(Path::new(&args_os().nth(1).unwrap()))?;

    if let Err(es) = open_data.run_checks() {
        for e in es {
            error!("{e}");
        }
    }

    // TODO cli disable flag
    let vateud8 = vateud8::get(env::var("VATSIM_OPEN_DATA_VATEUD8_URL").ok().as_deref()).unwrap();
    if let Err(es) = vateud8.check(&open_data) {
        for e in es {
            error!("{e}");
        }
    }

    println!("{}", serde_json::to_string_pretty(&open_data).unwrap());

    Ok(())
}
