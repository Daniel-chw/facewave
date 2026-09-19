// makes files visible to compiler and provides global types

pub mod image;
pub mod format;
pub mod encode;
pub mod decode;
pub mod stages;

// Image Types
#[derive(Debug, Clone)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub data: Vec<f32>,
}

// DWT Types
#[derive(Debug, Clone)]
pub struct HaarTransformed {
    pub w: usize,
    pub h: usize,
    pub levels: u8,
    pub data: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    LL,
    LH,
    HL,
    HH,
}

// Quantised Types
#[derive(Debug, Clone)]
pub struct Quantised {
    pub w: usize,
    pub h: usize,
    pub levels: u8,
    pub step: f32,
    pub data: Vec<i16>,
}


// Zerotree Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    ZeroTree,
    IsolatedZero,
    Positive,
    Negative,
}