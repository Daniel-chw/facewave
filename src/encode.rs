// Facial encode pipeline

use crate::Image;
use crate::stages::{symmtery, dwt, quantise, zerotree, arithmetic}

pub fn encode(image: &Image, dwt_depth_S: u8, dwt_depth_D: u8, quantise_step_S: f32, quantise_step_D: f32) -> Vec<u8> {

    let (s,d) = symmtery::split(image);

    let haar_s = dwt::haar(&s, dwt_depth_S);
    let haar_d = dwt::haar(&d, dwt_depth_D);

    let quantise_s = quantise::quantise(&haar_s, quantise_step_S);
    let quantise_d = quantise::quantise(&haar_d, quantise_step_D);

    let zerotree_s = zerotree::encode(&quantise_s);
    let zerotree_d = zerotree::encode(&quantise_d);

    let bytes_s = arithmetic::encode(&zerotree_s);
    let bytes_d = arithmetic::encode(&zerotree_d);

    todo!("pack header + bytes.s + bytes.d using format.rs")
}