use crate::{Quantised, Symbol, Band};

// -------------------- Helpers --------------------

// returns index for scales=[LH1,HL1,HH1,LH2, ...] for a given level+band
pub fn scale_index(level: u8, band: Band) -> usize {
    match band {
        Band::LH => (level as usize - 1) * 3,
        Band::HL => (level as usize - 1) * 3 + 1,
        Band::HH => (level as usize - 1) * 3 + 2,
        Band::LL => (3 * level) as usize,
    }
}

// returns level+band of some given pixel
pub fn level_and_band_of(x: usize, y: usize, w: usize, h: usize, levels: u8) -> (u8, Band) {
    let (mut cw, mut ch) = (w, h);
    for level in 1..=levels {
        let (hw, hh) = (cw / 2, ch / 2);
        if x < cw && y < ch && (x >= hw || y >= hh) {
            let band = match (x >= hw, y >= hh) {
                (true, false) => Band::LH,
                (false, true) => Band::HL,
                _ => Band::HH,
            };
            return (level, band);
        }
        cw = hw;
        ch = hh;
    }
    (levels, Band::LL)
}

// returns list of children for some pixel in zerotree
pub fn children_of(x: usize, y: usize, w: usize, h: usize, levels: u8) -> Vec<(usize, usize)>{
    let (level, band) = level_and_band_of(x,y,w,h,levels);
    let mut children = Vec::new();
    if band == Band::LL {
        let (cw, ch) = (w >> levels, h >> levels);
        children.extend([(x + cw, y), (x, y + ch), (x + cw, y + ch)]);
        children
    }
    else if level == 1 {
        children
    }
    else {
        children.extend([(2*x,2*y), (2*x + 1,2*y), (2*x,2*y + 1),(2*x + 1,2*y + 1)]);
        children
    }
}


// -------------------- Tempory --------------------

// TEMPORARY
pub fn build(q: &Quantised) -> Vec<Symbol> {
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
pub fn unbuild(symbols: &[Symbol], w: usize, h: usize, levels: u8, scales: Vec<f32>) -> Quantised {
    let data: Vec<i16> = symbols.iter().map(|s| match s {
        Symbol::ZeroTree => 0,
        Symbol::Positive => 1,
        Symbol::Negative => -1,
        Symbol::IsolatedZero => 0,
    }).collect();

    Quantised { w, h, levels, scales, data }
}

// -------------------- Testers --------------------
