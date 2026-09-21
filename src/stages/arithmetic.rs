use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;
use std::io::{Read, Write};
use crate::Symbol;

fn symbol_to_byte(s: Symbol) -> u8 {
    match s {
        Symbol::ZeroTree => 0,
        Symbol::IsolatedZero => 1,
        Symbol::Positive => 2,
        Symbol::Negative => 3,
        Symbol::RefineOne => 4,
        Symbol::RefineZero => 5,
    }
}

fn byte_to_symbol(b: u8) -> Symbol {
    match b {
        0 => Symbol::ZeroTree,
        1 => Symbol::IsolatedZero,
        2 => Symbol::Positive,
        3 => Symbol::Negative,
        4 => Symbol::RefineOne,
        5 => Symbol::RefineZero,
        _ => panic!("invalid symbol byte: {b}"),
    }
}

pub fn encode_symbols(symbols: &[Symbol]) -> Vec<u8> {
    let raw: Vec<u8> = symbols.iter().map(|&s| symbol_to_byte(s)).collect();

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw).unwrap();
    encoder.finish().unwrap()
}

pub fn decode_symbols(bytes: &[u8], count: usize) -> Vec<Symbol> {
    let mut decoder = ZlibDecoder::new(bytes);
    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw).unwrap();

    raw.into_iter().take(count).map(byte_to_symbol).collect()
}

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

    fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &c in &self.cum {
            h = (h ^ c as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
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
pub fn encode_adaptive(symbols: &[usize], model: &mut AdaptiveDist, mut trace: Option<&mut Vec<u64>>) -> Vec<u8> {
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
        if let Some(t) = trace.as_deref_mut() {
            t.push(model.fingerprint());
        }
    }

    // one bit pins the decoder inside [low, high], the padding reads as zero
    pending += 1;
    emit(&mut out, &mut pending, low >= QUARTER);
    out.finish()
}
