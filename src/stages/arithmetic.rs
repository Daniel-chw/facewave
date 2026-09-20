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
    }
}

fn byte_to_symbol(b: u8) -> Symbol {
    match b {
        0 => Symbol::ZeroTree,
        1 => Symbol::IsolatedZero,
        2 => Symbol::Positive,
        3 => Symbol::Negative,
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