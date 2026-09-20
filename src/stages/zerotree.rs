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


// true if every descendant of (x, y) is below the threshold
pub fn is_subtree_insignificant(q: &Quantised, x: usize, y: usize, t: u16) -> bool {
    children_of(x, y, q.w, q.h, q.levels).into_iter().all(|(cx, cy)| {
        q.data[cy * q.w + cx].unsigned_abs() < t && is_subtree_insignificant(q, cx, cy, t)
    })
}

// marks descendants of (x, y) as coded; the root itself keeps its own symbol
fn mark_subtree(covered: &mut [bool], x: usize, y: usize, w: usize, h: usize, levels: u8) {
    let mut stack = children_of(x, y, w, h, levels);
    while let Some((cx, cy)) = stack.pop() {
        covered[cy * w + cx] = true;
        stack.extend(children_of(cx, cy, w, h, levels));
    }
}


// -------------------- Zerotree --------------------

const THRESHOLD: u16 = 1;
const REPRESENTATIVE: i16 = THRESHOLD as i16;

pub fn symbol_count(w: usize, h: usize, _dwt_depth: u8) -> usize {
    w * h
}

pub fn dominant_pass(q: &Quantised) -> Vec<Symbol> {

    let mut out = Vec::new();
    let mut covered = vec![false; q.w * q.h];
    let t = THRESHOLD;

    for (x0,y0,x1,y1) in zerotree_traversal(q.w, q.h, q.levels) {
        for y in y0..y1 {
            for x in x0..x1 {

                if covered[y * q.w + x] { continue; }
                let p = q.data[y*q.w+x];

                if p.unsigned_abs() >= t {
                    if p > 0 {out.push(Symbol::Positive);}
                    else {out.push(Symbol::Negative);}
                }
                else {
                    if is_subtree_insignificant(q, x, y, t){
                        mark_subtree(&mut covered, x, y, q.w, q.h, q.levels);
                        out.push(Symbol::ZeroTree);
                    }
                    else {
                        out.push(Symbol::IsolatedZero);
                    }
                }
            }
        }
    }
    out
}

pub fn dominant_unpass(symbols: &[Symbol], w: usize, h: usize, levels: u8, scales: Vec<f32>) -> Quantised {
    let mut data = vec![0i16; w * h];
    let mut covered = vec![false; w * h];
    let mut next = 0;

    'scan: for (x0, y0, x1, y1) in zerotree_traversal(w, h, levels) {
        for y in y0..y1 {
            for x in x0..x1 {

                if covered[y * w + x] { continue; }

                // truncated stream: leave the rest zero
                if next >= symbols.len() { break 'scan; }
                let symbol = symbols[next];
                next += 1;

                if symbol == Symbol::Positive || symbol == Symbol::Negative {
                    if symbol == Symbol::Positive {data[y*w+x] = REPRESENTATIVE;}
                    else {data[y*w+x] = -REPRESENTATIVE;}
                }
                else {
                    if symbol == Symbol::ZeroTree {
                        mark_subtree(&mut covered, x, y, w, h, levels);
                    }
                }
            }
        }
    }

    Quantised { w, h, levels, scales, data }
}

pub fn subordinate_pass(q: &Quantised, t: u16, sig_list: &[(usize, usize)]) -> Vec<Symbol> {
    let mut out: Vec<Symbol> = Vec::new();
    for (x,y) in sig_list {
        if q.data[y*q.w + x].unsigned_abs() & t != 0 {
            out.push(Symbol::RefineOne);
        }
        else {
            out.push(Symbol::RefineZero);
        }
    }
    out
}

pub fn subordinate_unpass(q: &mut Quantised, t: u16, sig_list: &[(usize, usize)], symbols: &[Symbol]) {
    let mut next = 0;
    for (x,y) in sig_list {

        if next >= symbols.len() { break; }
        let symbol = symbols[next];
        next += 1;

        if symbol == Symbol::RefineOne {
            let v = q.data[y*q.w + x];
            let mag = (v.unsigned_abs() | t) as i16;

            if v < 0 {q.data[y*q.w + x] = -mag;}
            else {q.data[y*q.w + x] = mag;}
        }
    }
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

#[test]
fn unbuild_replays_build_significance_map() {
    let (w, h, levels) = (16usize, 16usize, 3u8);

    // mixed magnitudes: some over threshold, some under, some zero
    let data: Vec<i16> = (0..w * h)
        .map(|i| match i % 7 {
            0 => 0,
            1 => 5,
            2 => -9,
            3 => 40,
            4 => -77,
            5 => 32,
            _ => -31,
        })
        .collect();
    let scales = vec![1.0; 3 * levels as usize + 1];
    let q = Quantised { w, h, levels, scales: scales.clone(), data: data.clone() };

    let symbols = dominant_pass(&q);
    assert!(symbols.len() <= symbol_count(w, h, levels), "symbol_count must bound build");

    let back = dominant_unpass(&symbols, w, h, levels, scales);
    assert_eq!((back.w, back.h, back.levels), (w, h, levels));

    // every significant coeff keeps its sign; every insignificant one decodes to zero
    for i in 0..w * h {
        let (orig, got) = (data[i], back.data[i]);
        if orig.unsigned_abs() >= THRESHOLD {
            assert_eq!(got.signum(), orig.signum(), "sign lost at {i}: {orig} -> {got}");
            assert_eq!(got.unsigned_abs(), THRESHOLD, "magnitude not representative at {i}");
        } else {
            assert_eq!(got, 0, "insignificant {orig} at {i} decoded as {got}");
        }
    }
}

#[test]
fn unbuild_of_truncated_stream_zero_fills() {
    let (w, h, levels) = (8usize, 8usize, 2u8);
    let q = test_quantised(w, h, levels, &[(7, 7), (0, 0), (3, 3)]);
    let scales = q.scales.clone();

    let symbols = dominant_pass(&q);
    let back = dominant_unpass(&symbols[..symbols.len() / 2], w, h, levels, scales);

    assert_eq!(back.data.len(), w * h, "must still be a full-size image");
}

#[test]
fn subordinate_unpass_inverts_subordinate_pass() {
    let (w, h, levels) = (8usize, 8usize, 2u8);
    let scales = vec![1.0; 3 * levels as usize + 1];

    // magnitudes in [64, 128), so 64 is the dominant threshold and 32..1 refine
    let data: Vec<i16> = (0..w * h)
        .map(|i| {
            let mag = 64 + (i as i16 % 64);
            if i % 2 == 0 { mag } else { -mag }
        })
        .collect();
    let q = Quantised { w, h, levels, scales: scales.clone(), data: data.clone() };

    let sig_list: Vec<(usize, usize)> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).collect();

    // decoder starts at the dominant threshold, then refines bit by bit
    let mut back = Quantised {
        w, h, levels, scales,
        data: data.iter().map(|v| v.signum() * 64).collect(),
    };
    for bit in [32u16, 16, 8, 4, 2, 1] {
        let refinements = subordinate_pass(&q, bit, &sig_list);
        assert_eq!(refinements.len(), sig_list.len());
        subordinate_unpass(&mut back, bit, &sig_list, &refinements);
    }

    // all refinement bits restored: reconstruction is exact
    for i in 0..w * h {
        assert_eq!(back.data[i], data[i], "at {i}");
    }
}

#[test]
fn subordinate_unpass_tolerates_short_symbol_stream() {
    let (w, h, levels) = (8usize, 8usize, 2u8);
    let q = test_quantised(w, h, levels, &[(1, 1), (5, 5)]);
    let sig_list = [(1usize, 1usize), (5, 5)];

    let mut back = Quantised { w, h, levels, scales: q.scales.clone(), data: vec![1i16; w * h] };
    subordinate_unpass(&mut back, 32, &sig_list, &[Symbol::RefineOne]);

    assert_eq!(back.data[1 * w + 1], 33, "first entry refined");
    assert_eq!(back.data[5 * w + 5], 1, "second entry left at coarse value");
}
