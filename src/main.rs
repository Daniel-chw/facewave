// entry point for binary crate

mod cli;

use std::error::Error;
use std::fs;

use clap::Parser;
use cli::{Cli, Command, Encoded, fail, load_and_encode};

fn main() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::Encode {
            input,
            output,
            params,
        } => {
            let bytes = load_and_encode(&input, &params)?.bytes;
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
        Command::Eval { input, params } => {
            let Encoded {
                img,
                settings: (levels_s, levels_d, step_s, step_d),
                bytes,
            } = load_and_encode(&input, &params)?;
            let out = facewave::decode::decode(&bytes);

            let bpp = 8.0 * bytes.len() as f64 / (img.w * img.h) as f64;
            let psnr = facewave::image::psnr(&img, &out);
            println!(
                "{input},{levels_s},{levels_d},{step_s},{step_d},{},{bpp:.3},{psnr:.2}",
                bytes.len()
            );
        }
    }

    Ok(())
}
