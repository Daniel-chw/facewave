use crate::Symbol;

// -------------------- Bit IO --------------------

struct BitWriter {
    bytes: Vec<u8>,
    cur: u8,
    nbits: u8,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter { bytes: Vec::new(), cur: 0, nbits: 0 }
    }

    fn put(&mut self, bit: bool) {
        self.cur = (self.cur << 1) | bit as u8;
        self.nbits += 1;
        if self.nbits == 8 {
            self.bytes.push(self.cur);
            self.cur = 0;
            self.nbits = 0;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.bytes.push(self.cur << (8 - self.nbits));
        }
        self.bytes
    }
}

struct BitReader<'a> {
    bytes: &'a [u8],
    pos: usize,
    nbits: u8,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        BitReader { bytes, pos: 0, nbits: 0 }
    }

    // past the end reads as zero: the flush is only a bit or two long and the
    // rest of the code value is implicitly zero-padded
    fn get(&mut self) -> bool {
        let byte = self.bytes.get(self.pos).copied().unwrap_or(0);
        let bit = (byte >> (7 - self.nbits)) & 1 == 1;
        self.nbits += 1;
        if self.nbits == 8 {
            self.nbits = 0;
            self.pos += 1;
        }
        bit
    }
}

fn put_varint(out: &mut Vec<u8>, mut n: u64) {
    while n >= 0x80 {
        out.push(n as u8 | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}

fn take_varint(bytes: &[u8], at: &mut usize) -> u64 {
    let mut n = 0u64;
    let mut shift = 0;
    loop {
        let b = bytes[*at];
        *at += 1;
        n |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            return n;
        }
        shift += 7;
    }
}

// -------------------- Models / Distribtion --------------------

pub const MAX_TOTAL: u32 = 1 << 16;

#[derive(Debug, Clone)]
pub struct AdaptiveDist {
    cum: Vec<u32>,
    cap: u32,
}

// builds and maintains prob distrabution for encoding, and this updates regularly
impl AdaptiveDist {
    pub fn new(n: usize, cap: u32) -> Self {
        AdaptiveDist { cum: (0..=n as u32).collect(), cap }
    }

    fn cum(&self, i: usize) -> u64 {
        self.cum[i] as u64
    }

    fn total(&self) -> u64 {
        *self.cum.last().unwrap() as u64
    }

    fn lookup(&self, target: u64) -> usize {
        self.cum[1..].iter().position(|&c| target < c as u64).expect("target within total")
    }

    fn halve(&mut self) {
        let mut total = 0;
        for i in 1..self.cum.len() {
            let f = self.cum[i] - self.cum[i - 1];
            total += f.div_ceil(2);
            self.cum[i] = total;
        }
    }

    fn update(&mut self, s: usize) {

        for c in &mut self.cum[s + 1..] {
            *c += 1;
        }

        if *self.cum.last().unwrap() > self.cap {
            self.halve();
        }
    }

}

// -------------------- Coder --------------------

const HALF: u32 = 1 << 31;
const QUARTER: u32 = 1 << 30;
const THREE_QUARTERS: u32 = 3 << 30;

fn narrow(low: &mut u32, high: &mut u32, cum_low: u64, cum_high: u64, total: u64) {
    let base = *low as u64;
    let range = (*high - *low) as u64 + 1;
    *high = (base + range * cum_high / total - 1) as u32;
    *low = (base + range * cum_low / total) as u32;
}

// encode :: [Sym], prob dist -> BitStream
pub fn encode_adaptive(symbols: &[usize], model: &mut AdaptiveDist) -> Vec<u8> {
    let mut out = BitWriter::new();
    let mut low: u32 = 0;
    let mut high: u32 = u32::MAX;
    let mut pending: u64 = 0;

    fn emit(out: &mut BitWriter, pending: &mut u64, bit: bool) {
        out.put(bit);
        for _ in 0..*pending {
            out.put(!bit);
        }
        *pending = 0;
    }

    for &s in symbols {
        let total = model.total();
        narrow(&mut low, &mut high, model.cum(s), model.cum(s + 1), total);

        loop {
            if high < HALF {
                emit(&mut out, &mut pending, false);
            } else if low >= HALF {
                emit(&mut out, &mut pending, true);
                low -= HALF;
                high -= HALF;
            } else if low >= QUARTER && high < THREE_QUARTERS {
                pending += 1;
                low -= QUARTER;
                high -= QUARTER;
            } else {
                break;
            }
            low <<= 1;
            high = (high << 1) | 1;
        }

        model.update(s);
    }

    // one bit pins the decoder inside [low, high], the padding reads as zero
    pending += 1;
    emit(&mut out, &mut pending, low >= QUARTER);
    out.finish()
}

pub fn decode_adaptive(
    bytes: &[u8],
    count: usize,
    model: &mut AdaptiveDist,
) -> Vec<usize> {
    let mut input = BitReader::new(bytes);
    let mut low: u32 = 0;
    let mut high: u32 = u32::MAX;
    let mut value: u32 = 0;
    for _ in 0..32 {
        value = (value << 1) | input.get() as u32;
    }

    let mut symbols = Vec::with_capacity(count);
    for _ in 0..count {
        let total = model.total();
        let range = (high - low) as u64 + 1;
        let target = (((value - low) as u64 + 1) * total - 1) / range;
        let s = model.lookup(target);

        narrow(&mut low, &mut high, model.cum(s), model.cum(s + 1), total);
        symbols.push(s);

        loop {
            if high < HALF {
                // do nothing
            } else if low >= HALF {
                low -= HALF;
                high -= HALF;
                value -= HALF;
            } else if low >= QUARTER && high < THREE_QUARTERS {
                low -= QUARTER;
                high -= QUARTER;
                value -= QUARTER;
            } else {
                break;
            }
            low <<= 1;
            high = (high << 1) | 1;
            value = (value << 1) | input.get() as u32;
        }

        model.update(s);
    }

    symbols
}

// -------------------- EZW Symbols --------------------

// EZW has two passes thus two alphabets
// must be passed as ccontext into encoder and decoders to handle EZW
const CONTEXT_CAP: u32 = 1 << 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Dominant,
    Refinement,
}

impl Context {
    pub fn of(s: Symbol) -> Self {
        match s {
            Symbol::ZeroTree | Symbol::IsolatedZero | Symbol::Positive | Symbol::Negative => {
                Context::Dominant
            }
            Symbol::RefineOne | Symbol::RefineZero => Context::Refinement,
        }
    }

    fn model(self) -> AdaptiveDist {
        match self {
            Context::Dominant => AdaptiveDist::new(4, CONTEXT_CAP),
            Context::Refinement => AdaptiveDist::new(2, CONTEXT_CAP),
        }
    }

    fn index_of(self, s: Symbol) -> usize {
        match (self, s) {
            (Context::Dominant, Symbol::ZeroTree) => 0,
            (Context::Dominant, Symbol::IsolatedZero) => 1,
            (Context::Dominant, Symbol::Positive) => 2,
            (Context::Dominant, Symbol::Negative) => 3,
            (Context::Refinement, Symbol::RefineZero) => 0,
            (Context::Refinement, Symbol::RefineOne) => 1,
            _ => panic!("{s:?} does not belong to {self:?}"),
        }
    }

    fn symbol_at(self, i: usize) -> Symbol {
        match (self, i) {
            (Context::Dominant, 0) => Symbol::ZeroTree,
            (Context::Dominant, 1) => Symbol::IsolatedZero,
            (Context::Dominant, 2) => Symbol::Positive,
            (Context::Dominant, 3) => Symbol::Negative,
            (Context::Refinement, 0) => Symbol::RefineZero,
            (Context::Refinement, 1) => Symbol::RefineOne,
            _ => panic!("index {i} out of range for {self:?}"),
        }
    }
}

// each stream in the order its pass emits it
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Streams {
    pub dominant: Vec<Symbol>,
    pub refinement: Vec<Symbol>,
}

pub fn encode_symbols(symbols: &[Symbol]) -> Vec<u8> {
    let mut dominant = Vec::new();
    let mut refinement = Vec::new();
    for &s in symbols {
        let ctx = Context::of(s);
        match ctx {
            Context::Dominant => dominant.push(ctx.index_of(s)),
            Context::Refinement => refinement.push(ctx.index_of(s)),
        }
    }

    let coded_d = encode_adaptive(&dominant, &mut Context::Dominant.model());
    let coded_r = encode_adaptive(&refinement, &mut Context::Refinement.model());

    let mut out = Vec::with_capacity(coded_d.len() + coded_r.len() + 12);
    put_varint(&mut out, dominant.len() as u64);
    put_varint(&mut out, refinement.len() as u64);
    put_varint(&mut out, coded_d.len() as u64);
    out.extend_from_slice(&coded_d);
    out.extend_from_slice(&coded_r);
    out
}

pub fn decode_symbols(bytes: &[u8]) -> Streams {
    let mut at = 0;
    let n_dominant = take_varint(bytes, &mut at) as usize;
    let n_refinement = take_varint(bytes, &mut at) as usize;
    let dominant_len = take_varint(bytes, &mut at) as usize;

    let (coded_d, coded_r) = bytes[at..].split_at(dominant_len);

    let to_symbols = |indices: Vec<usize>, ctx: Context| {
        indices.into_iter().map(|i| ctx.symbol_at(i)).collect()
    };

    Streams {
        dominant: to_symbols(
            decode_adaptive(coded_d, n_dominant, &mut Context::Dominant.model()),
            Context::Dominant,
        ),
        refinement: to_symbols(
            decode_adaptive(coded_r, n_refinement, &mut Context::Refinement.model()),
            Context::Refinement,
        ),
    }
}

// -------------------- Tests --------------------

#[cfg(test)]
mod adaptive_tests {
    use super::*;

    struct Rng(u64);

    impl Rng {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next_u64() % n
        }
    }

    fn draw(rng: &mut Rng, len: usize, weights: &[u32]) -> Vec<usize> {
        let total: u64 = weights.iter().map(|&w| w as u64).sum();
        (0..len)
            .map(|_| {
                let mut t = rng.below(total);
                weights
                    .iter()
                    .position(|&w| {
                        if t < w as u64 {
                            true
                        } else {
                            t -= w as u64;
                            false
                        }
                    })
                    .unwrap()
            })
            .collect()
    }

    // fresh models both sides: the decoder has to rebuild the same table
    fn roundtrip(symbols: &[usize], n: usize, cap: u32) {
        let encoded = encode_adaptive(symbols, &mut AdaptiveDist::new(n, cap));
        let decoded = decode_adaptive(&encoded, symbols.len(), &mut AdaptiveDist::new(n, cap));
        assert_eq!(decoded, symbols, "n = {n}, cap = {cap}, len = {}", symbols.len());
    }

    fn roundtrips_random_sequences(n: usize, seed: u64) {
        let mut rng = Rng(seed);

        for _ in 0..100 {
            let len = rng.below(10_001) as usize;
            // Sources range from flat to nearly degenerate, so the model has
            // to cope with both no signal and a hard-won skew.
            let source: Vec<u32> = (0..n).map(|_| 1 + rng.below(1_000) as u32).collect();
            // Caps from "halves constantly" to "never halves".
            let cap = [2 * n as u32, 64, 1 << 10, MAX_TOTAL][rng.below(4) as usize];

            roundtrip(&draw(&mut rng, len, &source), n, cap);
        }
    }

    #[test]
    fn roundtrips_random_sequences_over_4_symbols() {
        roundtrips_random_sequences(4, 0xa5a5_1234_dead_0001);
    }

    #[test]
    fn roundtrips_random_sequences_over_6_symbols() {
        roundtrips_random_sequences(6, 0x5a5a_4321_beef_0002);
    }

    #[test]
    fn roundtrips_edge_cases() {
        for n in [2, 4, 6] {
            for cap in [2 * n as u32, 64, MAX_TOTAL] {
                roundtrip(&[], n, cap);
                for s in 0..n {
                    roundtrip(&[s], n, cap);
                    roundtrip(&vec![s; 10_000], n, cap);
                }
                roundtrip(&(0..10_000).map(|i| i % n).collect::<Vec<_>>(), n, cap);
            }
        }
    }

    #[test]
    fn adaptation_converges_on_a_skewed_source() {
        let mut rng = Rng(0xfeed_face_0000_1234);
        // 6 symbols, one dominant: entropy is ~0.42 bits/symbol.
        let weights = [60_000, 200, 200, 200, 200, 200];
        let symbols = draw(&mut rng, 20_000, &weights);

        let adaptive = encode_adaptive(&symbols, &mut AdaptiveDist::new(6, MAX_TOTAL));

        let bits_per_symbol = 8.0 * adaptive.len() as f64 / symbols.len() as f64;
        assert!(bits_per_symbol < 0.50, "got {bits_per_symbol} bits/symbol");
        // the whole point: it learns the skew without being told it, so it must
        // beat the log2(6) bits a flat model would spend, by a wide margin
        let flat = 6f64.log2() * symbols.len() as f64 / 8.0;
        assert!((adaptive.len() as f64) * 4.0 < flat, "adaptive {} vs flat {flat:.0}", adaptive.len());

        assert_eq!(decode_adaptive(&adaptive, symbols.len(), &mut AdaptiveDist::new(6, MAX_TOTAL)), symbols);
    }

    #[test]
    fn a_tight_cap_costs_precision_but_stays_correct() {
        let mut rng = Rng(0xcafe_d00d_0000_4321);
        let weights = [60_000, 200, 200, 200, 200, 200];
        let symbols = draw(&mut rng, 20_000, &weights);

        let tight = encode_adaptive(&symbols, &mut AdaptiveDist::new(6, 16));
        let loose = encode_adaptive(&symbols, &mut AdaptiveDist::new(6, MAX_TOTAL));

        assert!(tight.len() > loose.len(), "tight {} vs loose {}", tight.len(), loose.len());
        assert_eq!(decode_adaptive(&tight, symbols.len(), &mut AdaptiveDist::new(6, 16)), symbols);
    }
}

#[cfg(test)]
mod symbol_tests {
    use super::*;
    use crate::Symbol::*;

    // splits the way zerotree::unbuild does, the contract the pipeline relies on
    fn split(symbols: &[Symbol]) -> Streams {
        let (dominant, refinement) = symbols
            .iter()
            .partition(|&&s| !matches!(s, RefineOne | RefineZero));
        Streams { dominant, refinement }
    }

    fn roundtrip(symbols: &[Symbol]) {
        let decoded = decode_symbols(&encode_symbols(symbols));
        assert_eq!(decoded, split(symbols), "len = {}", symbols.len());
    }

    #[test]
    fn roundtrips_random_streams() {
        let mut rng = Rng(0x00c0_ffee_0000_0001);
        let alphabet = [ZeroTree, IsolatedZero, Positive, Negative, RefineOne, RefineZero];

        for _ in 0..200 {
            let len = rng.below(10_001) as usize;
            let symbols: Vec<Symbol> =
                (0..len).map(|_| alphabet[rng.below(6) as usize]).collect();
            roundtrip(&symbols);
        }
    }

    #[test]
    fn roundtrips_edge_cases() {
        roundtrip(&[]);
        for s in [ZeroTree, IsolatedZero, Positive, Negative, RefineOne, RefineZero] {
            roundtrip(&[s]);
            roundtrip(&vec![s; 10_000]);
        }
        // one context entirely absent
        roundtrip(&vec![ZeroTree; 5_000]);
        roundtrip(&vec![RefineOne; 5_000]);
    }

    #[test]
    fn splitting_contexts_beats_one_shared_table() {
        let mut rng = Rng(0x00c0_ffee_0000_0003);
        // Skewed dominant symbols next to near-random refinement bits: the
        // case where a shared table loses, since the refinements flatten it.
        let mut symbols = Vec::new();
        for _ in 0..20 {
            for _ in 0..1_000 {
                symbols.push(if rng.below(20) == 0 { Positive } else { ZeroTree });
            }
            for _ in 0..1_000 {
                symbols.push(if rng.below(2) == 0 { RefineOne } else { RefineZero });
            }
        }

        let split_size = encode_symbols(&symbols).len();

        let shared: Vec<usize> = symbols
            .iter()
            .map(|&s| match s {
                ZeroTree => 0,
                IsolatedZero => 1,
                Positive => 2,
                Negative => 3,
                RefineOne => 4,
                RefineZero => 5,
            })
            .collect();
        let shared_size = encode_adaptive(&shared, &mut AdaptiveDist::new(6, CONTEXT_CAP)).len();

        assert!(split_size < shared_size, "split {split_size} vs shared {shared_size}");
    }
}
