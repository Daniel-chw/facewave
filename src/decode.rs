// Facial decode pipeline

use crate::format;
use crate::stages::{arithmetic, dwt, quantise, symmetry, zerotree};
use crate::Image;

pub fn decode(bytes: &[u8]) -> Image {
    let (w, h, dwt_depth_s, dwt_depth_d, quantise_step_s, quantise_step_d, bytes_s, bytes_d) = format::unpack(bytes);

    let symbol_count_s = zerotree::symbol_count(w, h, dwt_depth_s);
    let symbol_count_d = zerotree::symbol_count(w, h, dwt_depth_d);

    let zerotree_s = arithmetic::decode(&bytes_s, symbol_count_s);
    let zerotree_d = arithmetic::decode(&bytes_d, symbol_count_d);

    let quantise_s = zerotree::unscan(&zerotree_s, w, h, dwt_depth_s, quantise_step_s);
    let quantise_d = zerotree::unscan(&zerotree_d, w, h, dwt_depth_d, quantise_step_d);

    let haar_s = quantise::dequantise(&quantise_s);
    let haar_d = quantise::dequantise(&quantise_d);

    let s = dwt::inverse_haar(&haar_s);
    let d = dwt::inverse_haar(&haar_d);

    symmetry::merge(&s, &d)
}
