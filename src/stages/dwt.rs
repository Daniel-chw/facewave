use crate::{HaarTransformed, Image};

pub fn haar_1d(row: &[f32]) -> Vec<f32> {

    let sqrt2 = 2.0_f32.sqrt();
    let averages = row.chunks_exact(2).map(|p| (p[0] + p[1]) / sqrt2);
    let details = row.chunks_exact(2).map(|p| (p[0] - p[1]) / sqrt2);

    averages.chain(details).collect()
}

pub fn inverse_haar_1d(coeffs: &[f32]) -> Vec<f32> {

    let sqrt2 = 2.0_f32.sqrt();
    let (averages, details) = coeffs.split_at(coeffs.len() / 2);

    averages
        .iter()
        .zip(details.iter())
        .flat_map(|(a, d)| [(a + d) / sqrt2, (a - d) / sqrt2])
        .collect()
}


pub fn haar(_img: &Image, _levels: u8) -> HaarTransformed {
    todo!()
}

pub fn inverse_haar(_coeffs: &HaarTransformed) -> Image {
    todo!()
}


#[test]
fn haar_1d_roundtrip_and_energy() {
    let row = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

    let coeffs = haar_1d(&row);
    let out = inverse_haar_1d(&coeffs);

    for (a, b) in row.iter().zip(out.iter()) {
        assert!((a - b).abs() < 1e-5, "mismatch: {a} vs {b}");
    }

    let e_in: f32 = row.iter().map(|v| v * v).sum();
    let e_out: f32 = coeffs.iter().map(|v| v * v).sum();
    assert!((e_in - e_out).abs() < 1e-4);
}