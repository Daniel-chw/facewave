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
