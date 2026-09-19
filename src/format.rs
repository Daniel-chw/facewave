// handles .face data conversions

#[allow(clippy::too_many_arguments)]
pub fn pack(w: usize, h: usize, levels_s: u8, levels_d: u8, step_s: f32, step_d: f32, bytes_s: &[u8], bytes_d: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    out.extend_from_slice(&(w as u32).to_le_bytes());
    out.extend_from_slice(&(h as u32).to_le_bytes());

    out.push(levels_s);
    out.push(levels_d);

    out.extend_from_slice(&step_s.to_le_bytes());
    out.extend_from_slice(&step_d.to_le_bytes());

    out.extend_from_slice(&(bytes_s.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes_s);

    out.extend_from_slice(&(bytes_d.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes_d);

    out
}

pub fn unpack(bytes: &[u8]) -> (usize, usize, u8, u8, f32, f32, Vec<u8>, Vec<u8>) {
    let w = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let h = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;

    let levels_s = bytes[8];
    let levels_d = bytes[9];

    let step_s = f32::from_le_bytes(bytes[10..14].try_into().unwrap());
    let step_d = f32::from_le_bytes(bytes[14..18].try_into().unwrap());

    let len_s = u32::from_le_bytes(bytes[18..22].try_into().unwrap()) as usize;
    let end_s = 22 + len_s;
    let bytes_s = bytes[22..end_s].to_vec();

    let len_d = u32::from_le_bytes(bytes[end_s..end_s + 4].try_into().unwrap()) as usize;
    let start_d = end_s + 4;
    let bytes_d = bytes[start_d..start_d + len_d].to_vec();

    (w, h, levels_s, levels_d, step_s, step_d, bytes_s, bytes_d)
}

#[test]
fn pack_unpack_roundtrip() {
    let s = vec![1u8, 2, 3, 4, 5];
    let d = vec![9u8, 8];

    let packed = pack(64, 32, 3, 2, 0.5, 2.25, &s, &d);
    let (w, h, levels_s, levels_d, step_s, step_d, out_s, out_d) = unpack(&packed);

    assert_eq!((w, h), (64, 32));
    assert_eq!((levels_s, levels_d, step_s, step_d), (3, 2, 0.5, 2.25));
    assert_eq!(out_s, s);
    assert_eq!(out_d, d);
}
