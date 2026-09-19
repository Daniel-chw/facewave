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


pub fn haar_2d_single_level(data: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut haar_row: Vec<f32> = Vec::with_capacity(w * h);

    for row in data.chunks(w) {
        haar_row.extend(haar_1d(row));
    }

    let mut out = haar_row.clone();

    for x in 0..w {
        let col: Vec<f32> = (0..h).map(|y| haar_row[y * w + x]).collect();

        for (y, v) in haar_1d(&col).into_iter().enumerate() {
            out[y * w + x] = v;
        }
    }

    out
}

pub fn haar_2d_single_level_inverse(data: &[f32], w: usize, h: usize) -> Vec<f32> {

    let mut haar_col: Vec<f32> = data.to_vec();
    for x in 0..w {
        let col: Vec<f32> = (0..h).map(|y| data[y * w + x]).collect();

        for (y, v) in haar_1d(&col).into_iter().enumerate() {
            haar_col[y * w + x] = v;
        }
    }

    let mut out = haar_col.clone();
    for (y, row) in haar_col.chunks(w).enumerate() {
        for (x, v) in inverse_haar_1d(row).into_iter().enumerate() {
            out[y * w + x] = v;
        }
    }

    out
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
#[test]
fn inverse_haar_1d_of_forward_is_identity() {
    let cases: Vec<Vec<f32>> = vec![
        vec![3.0, -5.0],
        vec![0.0; 8],
        vec![7.5; 6],
        vec![-1.0, 4.0, 0.5, -2.25, 100.0, -100.0, 0.0, 9.0],
        (0..64).map(|i| ((i * 37 % 17) as f32) - 8.0).collect(),
    ];

    for row in cases {
        let out = inverse_haar_1d(&haar_1d(&row));

        assert_eq!(out.len(), row.len());
        for (a, b) in row.iter().zip(out.iter()) {
            assert!((a - b).abs() < 1e-4, "mismatch: {a} vs {b} for {row:?}");
        }
    }
}


#[test]
fn single_level_roundtrip() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]; // 4x2
    let w = 4;
    let h = 2;

    let transformed = haar_2d_single_level(&data, w, h);
    let restored = haar_2d_single_level_inverse(&transformed, w, h);

    for (a, b) in data.iter().zip(restored.iter()) {
        assert!((a - b).abs() < 1e-4, "roundtrip mismatch: {a} vs {b}");
    }
}