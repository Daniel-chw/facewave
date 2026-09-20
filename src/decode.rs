// Facial decode pipeline

use crate::format;
use crate::stages::{arithmetic, dwt, quantise, symmetry, zerotree};
use crate::Image;

pub fn decode(bytes: &[u8]) -> Image {
    let (w, h, dwt_depth_s, dwt_depth_d, scales_s, scales_d, bytes_s, bytes_d) = format::unpack(bytes);

    // symmetry::split halves the width, so both half-images are w/2 wide;
    // symmetry::merge restores the full width at the end.
    let half_w = w / 2;

    let symbol_count_s = zerotree::symbol_count(half_w, h, dwt_depth_s);
    let symbol_count_d = zerotree::symbol_count(half_w, h, dwt_depth_d);

    let symbols_s = arithmetic::decode_symbols(&bytes_s, symbol_count_s);
    let symbols_d = arithmetic::decode_symbols(&bytes_d, symbol_count_d);

    let quantised_s = zerotree::unscan(&symbols_s, half_w, h, dwt_depth_s, scales_s);
    let quantised_d = zerotree::unscan(&symbols_d, half_w, h, dwt_depth_d, scales_d);

    let haar_s = quantise::dequantise(&quantised_s);
    let haar_d = quantise::dequantise(&quantised_d);

    let s = dwt::inverse_haar(&haar_s);
    let d = dwt::inverse_haar(&haar_d);

    symmetry::merge(&s, &d)
}
