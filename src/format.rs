// handles .face data conversions

pub fn pack(levels: u8, step_s: f32, step_d: f32, bytes_s: &[u8], bytes_d: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    out.push(levels);
    out.extend_from_slice(&step_s.to_le_bytes());
    out.extend_from_slice(&step_d.to_le_bytes());

    out.extend_from_slice(&(bytes_s.len() as u32).to_le_bytes());
    out.extend_from_slice(&bytes_s);

    out.extend_from_slice(&(bytes_d.len() as u32).to_le_bytes());
    out.extend_from_slice(&bytes_d);

    out
}

pub fn unpack(bytes: &[u8]) -> (u8, f32, f32, Vec<u8>, Vec<u8>) {
    let levels = bytes[0];
    let step_s = f32::from_le_bytes(bytes[1..5].try_into().unwrap());
    let step_d = f32::from_le_bytes(bytes[5..9].try_into().unwrap());

    let len_s = u32::from_le_bytes(bytes[9..13].try_into().unwrap()) as usize;
    let end_s = 13 + len_s;
    let bytes_s = bytes[13..end_s].to_vec();

    let len_d = u32::from_le_bytes(bytes[end_s..end_s + 4].try_into().unwrap()) as usize;
    let start_d = end_s + 4;
    let bytes_d = bytes[start_d..start_d + len_d].to_vec();

    (levels, step_s, step_d, bytes_s, bytes_d)
}

#[test]
fn pack_unpack_roundtrip() {
    let s = vec![1u8, 2, 3, 4, 5];
    let d = vec![9u8, 8];

    let packed = pack(3, 0.5, 2.25, &s, &d);
    let (levels, step_s, step_d, out_s, out_d) = unpack(&packed);

    assert_eq!((levels, step_s, step_d), (3, 0.5, 2.25));
    assert_eq!(out_s, s);
    assert_eq!(out_d, d);
}
