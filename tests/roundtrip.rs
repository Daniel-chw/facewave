use facewave::image::psnr;
use facewave::{decode::decode, encode::encode, Image};

// End-to-end encode -> decode on a synthetic image.
#[test]
fn encode_decode_roundtrip_is_plausible() {
    let (w, h) = (32usize, 32usize);

    // smooth gradient plus a hard edge, values in [0, 1]
    let data: Vec<f32> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let ramp = (x + y) as f32 / (w + h) as f32;
            let edge = if x > w / 2 { 0.25 } else { 0.0 };
            (ramp + edge).clamp(0.0, 1.0)
        })
        .collect();

    let original = Image { w, h, data };

    // one scale per detail band (3 per level) plus the final LL band
    let levels = 2u8;
    let scales = vec![0.01f32; 3 * levels as usize + 1];

    let bytes = encode(&original, levels, levels, scales.clone(), scales);
    assert!(!bytes.is_empty(), "encoder produced no bytes");

    let decoded = decode(&bytes).unwrap();

    assert_eq!((decoded.w, decoded.h), (original.w, original.h));
    assert_eq!(decoded.data.len(), original.data.len());
    assert!(
        decoded.data.iter().all(|v| v.is_finite()),
        "decoded image contains non-finite samples"
    );

    let quality = psnr(&original, &decoded);
    assert!(quality > 2.0, "PSNR too low: {quality} dB");
}

// ---------------------------------------------------------------------------
// The arithmetic stage against real EZW symbol streams.
// ---------------------------------------------------------------------------

use facewave::stages::{arithmetic, dwt, quantise, symmetry, zerotree};
use facewave::Symbol;

/// A few images with different amounts of structure, so the symbol streams
/// differ in shape rather than just in length.
fn test_images(w: usize, h: usize) -> Vec<(&'static str, Image)> {
    let gradient: Vec<f32> = (0..w * h)
        .map(|i| (i % w + i / w) as f32 / (w + h) as f32)
        .collect();

    let checker: Vec<f32> = (0..w * h)
        .map(|i| if ((i % w) / 4 + (i / w) / 4) % 2 == 0 { 0.2 } else { 0.8 })
        .collect();

    // deterministic pseudo-noise: worst case for the zerotree, since almost
    // nothing gets pruned
    let noise: Vec<f32> = (0..w * h)
        .map(|i| {
            let x = (i as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((x >> 33) % 1000) as f32 / 1000.0
        })
        .collect();

    let flat = vec![0.5f32; w * h];

    vec![
        ("gradient", Image { w, h, data: gradient }),
        ("checker", Image { w, h, data: checker }),
        ("noise", Image { w, h, data: noise }),
        ("flat", Image { w, h, data: flat }),
    ]
}

#[test]
fn real_symbol_streams_roundtrip_across_configurations() {
    let (w, h) = (64usize, 64usize);
    let mut total_symbols = 0usize;

    for (name, img) in test_images(w, h) {
        for levels in 1..=4u8 {
            for &step in &[0.015625f32, 0.0625, 0.25, 1.0] {
                // the same route encode::encode takes, so these are the
                // streams the pipeline actually produces
                let (s, _) = symmetry::split(&img);
                let haar = dwt::haar(&s, levels);
                let q = quantise::quantise(&haar, vec![step; 3 * levels as usize + 1]);
                let (t0, symbols) = zerotree::build(&q);

                let streams = arithmetic::decode_symbols(&arithmetic::encode_symbols(&symbols));

                let (dominant, refinement): (Vec<Symbol>, Vec<Symbol>) = symbols
                    .iter()
                    .partition(|&&s| !matches!(s, Symbol::RefineOne | Symbol::RefineZero));
                let case = format!("{name} levels={levels} step={step}");
                assert_eq!(streams.dominant, dominant, "dominant stream differs: {case}");
                assert_eq!(streams.refinement, refinement, "refinement stream differs: {case}");

                // and the streams still rebuild the exact coefficients
                let back = zerotree::unbuild_split(
                    &streams.dominant, &streams.refinement,
                    t0, q.w, q.h, levels, q.scales.clone(),
                );
                assert_eq!(back.data, q.data, "coefficients differ: {case}");

                total_symbols += symbols.len();
            }
        }
    }

    // guards against the loop silently coding nothing
    assert!(total_symbols > 100_000, "only {total_symbols} symbols exercised");
}
