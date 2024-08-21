use std::{env::args_os, io, path::Path};

use vatsim_open_data::OpenData;

fn main() -> Result<(), vatsim_open_data::Error> {
    tracing_subscriber::fmt().with_writer(io::stderr).init();

    // FIXME
    let open_data = OpenData::from_path(Path::new(&args_os().nth(1).unwrap()))?;

    println!("{}", serde_json::to_string_pretty(&open_data).unwrap());
    if let Err(es) = open_data.run_checks() {
        for e in es {
            eprintln!("{e}");
        }
    }

    Ok(())
}
