// entry point for binary crate

use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "tests/faces/face.jpg")]
    input: String,
    #[arg(long, default_value_t = 1)]
    levels_s: u8,
    #[arg(long, default_value_t = 1)]
    levels_d: u8,
    #[arg(long, default_value_t = 2.0)]
    step_s: f32,
    #[arg(long, default_value_t = 0.25)]
    step_d: f32,
    /// optional path to write the decoded image to
    #[arg(long)]
    output: Option<String>,
}

// quantise wants one step per band: 3 detail bands per level, plus LL
fn scales(step: f32, levels: u8) -> Vec<f32> {
    vec![step; 3 * levels as usize + 1]
}

fn main() {
    let a = Args::parse();

    let img = facewave::image::load_grayscale(&a.input).unwrap_or_else(|e| panic!("could not load {}: {e}", a.input));

    let bytes = facewave::encode::encode(&img, a.levels_s, a.levels_d, scales(a.step_s, a.levels_s), scales(a.step_d, a.levels_d));
    let out = facewave::decode::decode(&bytes);

    if let Some(path) = &a.output {
        facewave::image::save(&out, path)
            .unwrap_or_else(|e| panic!("could not save {path}: {e}"));
    }

      let bpp = 8.0 * bytes.len() as f64 / (img.w * img.h) as f64;
      println!("{},{},{},{},{},{:.3},{:.2}",
          a.levels_s, a.levels_d, a.step_s, a.step_d, bytes.len(), bpp,
          facewave::image::psnr(&img, &out));
}
