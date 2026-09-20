// Facial encode pipeline

use crate::Image;
use crate::format;
use crate::stages::{symmetry, dwt, quantise, zerotree, arithmetic};


pub fn encode(image: &Image, dwt_depth_s: u8, dwt_depth_d: u8, scales_s: Vec<f32>, scales_d: Vec<f32>) -> Vec<u8> {
    let (s, d) = symmetry::split(image);

    let haar_s = dwt::haar(&s, dwt_depth_s);
    let haar_d = dwt::haar(&d, dwt_depth_d);

    let quantised_s = quantise::quantise(&haar_s, scales_s);
    let quantised_d = quantise::quantise(&haar_d, scales_d);

    let (t0_s, symbols_s) = zerotree::build(&quantised_s);
    let (t0_d, symbols_d) = zerotree::build(&quantised_d);

    let bytes_s = arithmetic::encode_symbols(&symbols_s);
    let bytes_d = arithmetic::encode_symbols(&symbols_d);

    format::pack(image.w, image.h, dwt_depth_s, dwt_depth_d, t0_s, t0_d, &quantised_s.scales, &quantised_d.scales, &bytes_s, &bytes_d)
}    