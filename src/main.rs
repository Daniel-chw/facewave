// entry point for binary crate

fn main() {
    let original = facewave::image::load_grayscale("tests/faces/face.jpg")
        .expect("failed to load test image");

    let levels: u8 = 1;

    let n_scales = 3 * levels as usize + 1;
    let scales_s: Vec<f32> = vec![2.0; n_scales];
    let scales_d: Vec<f32> = vec![0.5; n_scales];

    let bytes = facewave::encode::encode(&original, levels, levels, scales_s, scales_d);
    let reconstructed = facewave::decode::decode(&bytes);

    // image::save will not create parent directories
    std::fs::create_dir_all("output").expect("could not create output dir");

    facewave::image::save(&original, "output/original.png").expect("save failed");
    facewave::image::save(&reconstructed, "output/output.png").expect("save failed");

    let quality = facewave::image::psnr(&original, &reconstructed);
    println!("PSNR: {quality} dB");
}