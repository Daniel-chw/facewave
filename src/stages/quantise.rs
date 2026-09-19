use crate::{Band, HaarTransformed, Quantised};

// -------------------- Helpers --------------------

fn quantise_quadrant(data: &[f32], out: &mut [i16], full_w: usize, x0: usize, y0: usize, x1: usize, y1: usize, scale: f32) {
    for y in y0..y1 {
        for x in x0..x1 {
            let i = y * full_w + x;
            out[i] = (data[i] / scale).round() as i16;
        }
    }
}

fn dequantise_quadrant(data: &[i16], out: &mut [f32], full_w: usize, x0: usize, y0: usize, x1: usize, y1: usize, scale: f32) {
    for y in y0..y1 {
        for x in x0..x1 {
            let i = y * full_w + x;
            out[i] = data[i] as f32 * scale;
        }
    }
}

fn scale_index(level: u8, band: Band) -> usize {
    match band {
        Band::LH => (level as usize - 1) * 3,
        Band::HL => (level as usize - 1) * 3 + 1,
        Band::HH => (level as usize - 1) * 3 + 2,
        Band::LL => (3 * level) as usize,
    }
}

// -------------------- (De/)Quantise --------------------


pub fn quantise(haar_transformed: &HaarTransformed, scales: Vec<f32>) -> Quantised {
    let w = haar_transformed.w;
    let h = haar_transformed.h;
    let data = &haar_transformed.data;
    let levels = haar_transformed.levels;

    let mut out = vec![0i16; w * h];
    let mut curr_w = w;
    let mut curr_h = h;

    for level in 1..=levels {
        let half_w = curr_w / 2;
        let half_h = curr_h / 2;

        let lh = scales[scale_index(level, Band::LH)];
        let hl = scales[scale_index(level, Band::HL)];
        let hh = scales[scale_index(level, Band::HH)];

        quantise_quadrant(data, &mut out, w, half_w, 0, curr_w, half_h, lh);
        quantise_quadrant(data, &mut out, w, 0, half_h, half_w, curr_h, hl);
        quantise_quadrant(data, &mut out, w, half_w, half_h, curr_w, curr_h, hh);

        curr_w = half_w;
        curr_h = half_h;
    }

    let ll = scales[scale_index(levels, Band::LL)];
    quantise_quadrant(data, &mut out, w, 0, 0, curr_w, curr_h, ll);

    Quantised { w, h, levels, scales: scales.to_vec(), data: out }
}



pub fn dequantise(q: &Quantised) -> HaarTransformed {
    let w = q.w;
    let h = q.h;
    let data = &q.data;
    let levels = q.levels;
    let scales = &q.scales;

    let mut out = vec![0.0f32; w * h];
    let mut curr_w = w;
    let mut curr_h = h;

    for level in 1..=levels {
        let half_w = curr_w / 2;
        let half_h = curr_h / 2;

        let lh = scales[scale_index(level, Band::LH)];
        let hl = scales[scale_index(level, Band::HL)];
        let hh = scales[scale_index(level, Band::HH)];

        dequantise_quadrant(data, &mut out, w, half_w, 0, curr_w, half_h, lh);
        dequantise_quadrant(data, &mut out, w, 0, half_h, half_w, curr_h, hl);
        dequantise_quadrant(data, &mut out, w, half_w, half_h, curr_w, curr_h, hh);

        curr_w = half_w;
        curr_h = half_h;
    }

    let ll = scales[scale_index(levels, Band::LL)];
    dequantise_quadrant(data, &mut out, w, 0, 0, curr_w, curr_h, ll);

    HaarTransformed { w, h, levels, data: out }
}

// -------------------- Tests --------------------

#[test]
fn quantise_dequantise_roundtrip_within_half_step() {
    let (w, h) = (8, 8);
    let levels = 2u8;
    let data: Vec<f32> = (0..w * h).map(|i| ((i * 37 % 101) as f32) - 50.0).collect();
    let ht = HaarTransformed { w, h, levels, data: data.clone() };

    // distinct step per band: 3 detail bands per level + LL
    let scales: Vec<f32> = (0..3 * levels as usize + 1).map(|i| 1.0 + i as f32).collect();

    let back = dequantise(&quantise(&ht, scales.clone()));

    assert_eq!((back.w, back.h, back.levels), (w, h, levels));

    // level 1 detail bands live outside the top-left 4x4; check against that band's step
    for y in 0..h {
        for x in 0..w {
            let (lvl_step_idx, _) = band_of(x, y, w, h, levels);
            let step = scales[lvl_step_idx];
            let (a, b) = (data[y * w + x], back.data[y * w + x]);
            assert!((a - b).abs() <= step / 2.0 + 1e-5, "({x},{y}) {a} vs {b} step {step}");
        }
    }
}

#[cfg(test)]
fn band_of(x: usize, y: usize, w: usize, h: usize, levels: u8) -> (usize, ()) {
    let (mut cw, mut ch) = (w, h);
    for level in 1..=levels {
        let (hw, hh) = (cw / 2, ch / 2);
        if x < cw && y < ch && (x >= hw || y >= hh) {
            let band = match (x >= hw, y >= hh) {
                (true, false) => Band::LH,
                (false, true) => Band::HL,
                _ => Band::HH,
            };
            return (scale_index(level, band), ());
        }
        cw = hw;
        ch = hh;
    }
    (scale_index(levels, Band::LL), ())
}
