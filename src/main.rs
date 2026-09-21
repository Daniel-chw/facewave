// entry point for binary crate

mod cli;

use std::error::Error;
use std::fs;

use clap::Parser;
use cli::{Cli, Command, fail, load_and_encode};

fn main() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::Encode {
            input,
            output,
            params,
        } => {
            let (_, bytes) = load_and_encode(&input, &params)?;
            fs::write(&output, &bytes)
                .map_err(|e| fail(format!("could not write {output}: {e}")))?;
            eprintln!("{input} -> {output}: {} bytes", bytes.len());
        }
        Command::Decode { input, output } => {
            let bytes =
                fs::read(&input).map_err(|e| fail(format!("could not read {input}: {e}")))?;
            let img = facewave::decode::decode(&bytes);
            facewave::image::save(&img, &output)
                .map_err(|e| fail(format!("could not save {output}: {e}")))?;
        }
    }

    Ok(())
}
