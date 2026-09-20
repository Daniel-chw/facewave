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

    let decoded = decode(&bytes);

    assert_eq!((decoded.w, decoded.h), (original.w, original.h));
    assert_eq!(decoded.data.len(), original.data.len());
    assert!(
        decoded.data.iter().all(|v| v.is_finite()),
        "decoded image contains non-finite samples"
    );

    let quality = psnr(&original, &decoded);
    assert!(quality > 2.0, "PSNR too low: {quality} dB");
}
