use crate::Image;

pub fn split(img: &Image) -> (Image, Image) {

    let w = img.w;
    let h = img.h;
    let w_half = w/2;
    let sqrt2 = 2.0_f32.sqrt();

    let mut s_data = Vec::with_capacity(h);
    let mut d_data = Vec::with_capacity(h);

    for row in img.data.chunks(w) {
        let row_mirror: Vec<f32> = row.iter().rev().copied().collect();

        s_data.extend(row.iter().zip(row_mirror.iter()).take(w_half).map(|(a,b)| (a+b)/sqrt2));
        d_data.extend(row.iter().zip(row_mirror.iter()).take(w_half).map(|(a,b)| (a-b)/sqrt2));

    }

    (Image { w: w_half, h, data: s_data }, Image { w: w_half, h, data: d_data })
    
}

pub fn merge(_s: &Image, _d: &Image) -> Image {
    todo!()
}

#[test]
fn split_preserves_total_energy() {
    let img = Image {
        w: 4,
        h: 2,
        data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
    };

    let (s, d) = split(&img);

    let original_energy: f32 = img.data.iter().map(|v| v * v).sum();
    let split_energy: f32 = s.data.iter().map(|v| v * v).sum::<f32>()
        + d.data.iter().map(|v| v * v).sum::<f32>();

    assert!(
        (original_energy - split_energy).abs() < 1e-4,
        "energy not preserved: original={original_energy}, split={split_energy}"
    );
}