// Facial encode pipeline

use crate::Image;
use crate::stages::{symmetry, dwt, quantise, zerotree, arithmetic};

pub fn encode(image: &Image, dwt_depth_s: u8, dwt_depth_d: u8, quantise_step_s: f32, quantise_step_d: f32) -> Vec<u8> {

    let (s,d) = symmetry::split(image);

    let haar_s = dwt::haar(&s, dwt_depth_s);
    let haar_d = dwt::haar(&d, dwt_depth_d);

    let quantise_s = quantise::quantise(&haar_s, quantise_step_s);
    let quantise_d = quantise::quantise(&haar_d, quantise_step_d);

    let zerotree_s = zerotree::encode(&quantise_s);
    let zerotree_d = zerotree::encode(&quantise_d);

    let bytes_s = arithmetic::encode(&zerotree_s);
    let bytes_d = arithmetic::encode(&zerotree_d);

    todo!("pack header + bytes.s + bytes.d using format.rs")
}