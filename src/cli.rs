// command line arguments and shared helpers for the binary

use std::error::Error;
use std::fmt;
use std::fs;

use clap::{Args, Parser, Subcommand};
use facewave::Image;
use facewave::presets::PRESETS;

#[derive(Parser)]
#[command(version, about = "Wavelet image codec for aligned face photos")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Encode an image into a .face file
    Encode {
        input: String,
        #[arg(short, long)]
        output: String,
        #[command(flatten)]
        params: Params,
    },
    /// Decode a .face file into an image
    Decode {
        input: String,
        #[arg(short, long)]
        output: String,
    },
    /// Encode and decode in memory, print one CSV row of stats
    Eval {
        input: String,
        #[command(flatten)]
        params: Params,
    },
    /// Print the header of a .face file
    Info { input: String },
}

#[derive(Args)]
pub struct Params {
    /// Quality preset, 1 (smallest) to 10 (best)
    #[arg(short, long, default_value_t = 6, value_parser = clap::value_parser!(u8).range(1..=10))]
    quality: u8,
    #[arg(long)]
    levels_s: Option<u8>,
    #[arg(long)]
    levels_d: Option<u8>,
    #[arg(long, value_parser = positive_step)]
    step_s: Option<f32>,
    #[arg(long, value_parser = positive_step)]
    step_d: Option<f32>,
}

impl Params {
    pub fn resolve(&self) -> (u8, u8, f32, f32) {
        let (levels_s, levels_d, step_s, step_d) = PRESETS[self.quality as usize - 1];
        (
            self.levels_s.unwrap_or(levels_s),
            self.levels_d.unwrap_or(levels_d),
            self.step_s.unwrap_or(step_s),
            self.step_d.unwrap_or(step_d),
        )
    }
}

// quantise divides by the step, so zero, negative or NaN give garbage
fn positive_step(s: &str) -> Result<f32, String> {
    match s.parse::<f32>() {
        Ok(v) if v.is_finite() && v > 0.0 => Ok(v),
        Ok(_) => Err("must be a positive, finite number".into()),
        Err(e) => Err(e.to_string()),
    }
}

// main prints errors with Debug, so this keeps messages unquoted
pub struct CliError(String);

impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for CliError {}

pub fn fail(msg: impl Into<String>) -> CliError {
    CliError(msg.into())
}

// quantise wants one step per band: 3 detail bands per level, plus LL
fn scales(step: f32, levels: u8) -> Vec<f32> {
    vec![step; 3 * levels as usize + 1]
}

pub struct Encoded {
    pub img: Image,
    pub settings: (u8, u8, f32, f32),
    pub bytes: Vec<u8>,
}

pub fn load_and_encode(input: &str, params: &Params) -> Result<Encoded, CliError> {
    let img = facewave::image::load_grayscale(input)
        .map_err(|e| fail(format!("could not load {input}: {e}")))?;

    let settings = params.resolve();
    let (levels_s, levels_d, step_s, step_d) = settings;
    let bytes = facewave::encode::encode(
        &img,
        levels_s,
        levels_d,
        scales(step_s, levels_s),
        scales(step_d, levels_d),
    );

    Ok(Encoded {
        img,
        settings,
        bytes,
    })
}

// the format stores one step per band; the cli always writes them equal
fn steps(scales: &[f32]) -> String {
    match scales {
        [] => "none".into(),
        [first, rest @ ..] if rest.iter().all(|s| s == first) => first.to_string(),
        _ => scales
            .iter()
            .map(f32::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

pub fn info(input: &str) -> Result<(), CliError> {
    let bytes = fs::read(input).map_err(|e| fail(format!("could not read {input}: {e}")))?;
    let (w, h, levels_s, levels_d, t0_s, t0_d, scales_s, scales_d, bytes_s, bytes_d) =
        facewave::format::unpack(&bytes).map_err(|e| fail(format!("{input}: {e}")))?;

    println!("file        {input} ({} bytes)", bytes.len());
    println!("dimensions  {w} x {h}");
    println!("            levels  t0      payload  step");
    for (name, levels, t0, payload, scales) in [
        ("symmetric", levels_s, t0_s, &bytes_s, &scales_s),
        ("difference", levels_d, t0_d, &bytes_d, &scales_d),
    ] {
        println!(
            "{name:<10}  {levels:<6}  {t0:<6}  {:<7}  {}",
            payload.len(),
            steps(scales)
        );
    }

    Ok(())
}
