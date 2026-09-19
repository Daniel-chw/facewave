use crate::{Quantised, Symbol};

// TEMPORARY
pub fn encode(q: &Quantised) -> Vec<Symbol> {
    q.data.iter().map(|&v| {
        match v {
            0 => Symbol::ZeroTree,
            v if v > 0 => Symbol::Positive,
            _ => Symbol::Negative,
        }
    }).collect()
}

// TEMPORARY

pub fn symbol_count(w: usize, h: usize, _dwt_depth: u8) -> usize {
    w * h
}

// TEMPORARY
pub fn decode(symbols: &[Symbol], w: usize, h: usize, levels: u8, scales: Vec<f32>) -> Quantised {
    let data: Vec<i16> = symbols.iter().map(|s| match s {
        Symbol::ZeroTree => 0,
        Symbol::Positive => 1,
        Symbol::Negative => -1,
        Symbol::IsolatedZero => 0,
    }).collect();

    Quantised { w, h, levels, scales, data }
}