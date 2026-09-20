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

// checks if all children+subchildren are 0
pub fn is_subtree_zero(q: &Quantised, x: usize, y: usize) -> bool {
    let children = children_of(x, y, q.w, q.h, q.levels);
    if children.is_empty() {
        return true;
    }

    for (cx, cy) in children {
        if q.data[cy*q.w + cx] != 0 {
            return false;
        }
        if !is_subtree_zero(q, cx, cy) {
            return false;
        }
    }
    true
}

// band scan order, coarse to fine, like fig.5 in Jerome M. Shapiro EZW paper
// we create list of bands we must visit, [LL3, LH3, HL3, HH3, LH2, ...]
pub fn zerotree_traversal(w: usize, h: usize, levels: u8) -> Vec<(usize, usize, usize, usize)> {

    let mut bands = Vec::with_capacity(3 * levels as usize + 1);
    bands.push((0, 0, w >> levels, h >> levels));

    for level in (1..=levels).rev() {
        let (cw, ch) = (w >> (level - 1), h >> (level - 1));
        let (hw, hh) = (cw / 2, ch / 2);

        bands.push((hw, 0, cw, hh));   // LH
        bands.push((0, hh, hw, ch));   // HL
        bands.push((hw, hh, cw, ch));  // HH
    }

    bands
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

#[cfg(test)]
fn test_quantised(w: usize, h: usize, levels: u8, nonzero: &[(usize, usize)]) -> Quantised {
    let mut data = vec![0i16; w * h];
    for &(x, y) in nonzero {
        data[y * w + x] = 42;
    }
    let scales = vec![1.0; 3 * levels as usize + 1];
    Quantised { w, h, levels, scales, data }
}

#[test]
fn subtree_of_all_zeros_is_zero() {
    let q = test_quantised(8, 8, 2, &[]);
    for y in 0..q.h {
        for x in 0..q.w {
            assert!(is_subtree_zero(&q, x, y), "({x},{y})");
        }
    }
}

#[test]
fn leaves_are_always_zero_subtrees() {
    // every coeff nonzero: level 1 has no descendants, so still vacuously true
    let all: Vec<(usize, usize)> = (0..8).flat_map(|y| (0..8).map(move |x| (x, y))).collect();
    let q = test_quantised(8, 8, 2, &all);
    for y in 0..q.h {
        for x in 0..q.w {
            let (level, _) = level_and_band_of(x, y, q.w, q.h, q.levels);
            assert_eq!(is_subtree_zero(&q, x, y), level == 1, "({x},{y})");
        }
    }
}

#[test]
fn nonzero_leaf_falsifies_only_its_ancestors() {
    // 8x8, 2 levels: LL = 2x2 top-left, level-2 details fill the 4x4, level-1 details outside it.
    // (7,7) is a level-1 HH leaf; its ancestors are (3,3) [level-2 HH] and (1,1) [LL root].
    let q = test_quantised(8, 8, 2, &[(7, 7)]);

    assert!(!is_subtree_zero(&q, 1, 1), "LL root ancestor");
    assert!(!is_subtree_zero(&q, 3, 3), "level-2 HH parent");

    assert!(is_subtree_zero(&q, 7, 7), "the leaf itself is excluded");
    assert!(is_subtree_zero(&q, 0, 0), "sibling LL root");
    assert!(is_subtree_zero(&q, 3, 2), "level-2 HH, different subtree");
    assert!(is_subtree_zero(&q, 2, 2), "level-2 HH, different subtree");
}

#[test]
fn nonzero_direct_child_falsifies_parent() {
    // (3,3)'s children are (6,6), (7,6), (6,7), (7,7)
    for child in [(6, 6), (7, 6), (6, 7), (7, 7)] {
        let q = test_quantised(8, 8, 2, &[child]);
        assert!(!is_subtree_zero(&q, 3, 3), "child {child:?}");
    }
}

#[test]
fn ll_root_sees_its_three_detail_children() {
    // levels = 1: LL is 4x4, its children are the level-1 detail coeffs at +4
    for child in [(4, 0), (0, 4), (4, 4)] {
        let q = test_quantised(8, 8, 1, &[child]);
        assert!(!is_subtree_zero(&q, 0, 0), "child {child:?}");
    }
}

#[test]
fn traversal_covers_every_coeff_exactly_once() {
    for (w, h) in [(8, 8), (16, 8)] {
        for levels in 1..=3u8 {
            let bands = zerotree_traversal(w, h, levels);
            assert_eq!(bands.len(), 3 * levels as usize + 1);

            let mut seen = vec![0u8; w * h];
            for (x0, y0, x1, y1) in bands {
                for y in y0..y1 {
                    for x in x0..x1 {
                        seen[y * w + x] += 1;
                    }
                }
            }
            for (i, &n) in seen.iter().enumerate() {
                assert_eq!(n, 1, "({},{}) covered {n} times, w{w} h{h} l{levels}", i % w, i / w);
            }
        }
    }
}

#[test]
fn traversal_is_coarse_to_fine_and_matches_band_of() {
    let (w, h, levels) = (8usize, 8usize, 2u8);
    let bands = zerotree_traversal(w, h, levels);

    // expected level+band per slot: LL first, then level 2 LH/HL/HH, then level 1
    let expected = [
        (levels, Band::LL),
        (2, Band::LH), (2, Band::HL), (2, Band::HH),
        (1, Band::LH), (1, Band::HL), (1, Band::HH),
    ];

    for (&(x0, y0, x1, y1), &want) in bands.iter().zip(expected.iter()) {
        assert!(x0 < x1 && y0 < y1, "empty region {:?}", (x0, y0, x1, y1));
        for y in y0..y1 {
            for x in x0..x1 {
                assert_eq!(level_and_band_of(x, y, w, h, levels), want, "({x},{y})");
            }
        }
    }
}
