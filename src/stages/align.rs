// allignment <=> min ||D||_F ^2
// just have to rotate and shift image to try D

use crate::Image;
use crate::stages::symmetry;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alignment {
    pub shift: f32,
    pub angle: f32,
}

const MAX_SHIFT: f32 = 0.125;
const MAX_ANGLE: f32 = 15.0 * std::f32::consts::PI / 180.0;

// bilinear sample
fn sample(img: &Image, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (img.w - 1) as f32);
    let y = y.clamp(0.0, (img.h - 1) as f32);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(img.w - 1), (y0 + 1).min(img.h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);

    let at = |x: usize, y: usize| img.data[y * img.w + x];
    let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
    let bot = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
    top * (1.0 - fy) + bot * fy
}

// centre + R(angle)(p - centre) + (shift, 0)
pub fn apply(img: &Image, a: Alignment) -> Image {
    let cx = (img.w as f32 - 1.0) / 2.0;
    let cy = (img.h as f32 - 1.0) / 2.0;
    let dx = a.shift * img.w as f32;
    let (sin, cos) = a.angle.sin_cos();

    let mut data = Vec::with_capacity(img.w * img.h);
    for y in 0..img.h {
        for x in 0..img.w {
            let (u, v) = (x as f32 - cx, y as f32 - cy);
            data.push(sample(img, cx + dx + cos * u - sin * v, cy + sin * u + cos * v));
        }
    }

    Image { w: img.w, h: img.h, data }
}

pub fn d_energy(img: &Image) -> f32 {
    let (_, d) = symmetry::split(img);
    d.data.iter().map(|v| v * v).sum()
}

pub fn find(img: &Image) -> Alignment {
    let mut best = Alignment { shift: 0.0, angle: 0.0 };
    let mut best_e = d_energy(img);

    let steps = 16;
    let (mut shift_r, mut angle_r) = (MAX_SHIFT, MAX_ANGLE);

    for _ in 0..4 {
        let centre = best;
        for i in -steps..=steps {
            for j in -steps..=steps {
                let a = Alignment {
                    shift: (centre.shift + shift_r * i as f32 / steps as f32).clamp(-MAX_SHIFT, MAX_SHIFT),
                    angle: (centre.angle + angle_r * j as f32 / steps as f32).clamp(-MAX_ANGLE, MAX_ANGLE),
                };
                let e = d_energy(&apply(img, a));
                if e < best_e {
                    best_e = e;
                    best = a;
                }
            }
        }
        shift_r *= 2.0 / steps as f32;
        angle_r *= 2.0 / steps as f32;
    }

    best
}
