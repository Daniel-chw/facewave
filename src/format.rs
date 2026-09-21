// handles .face data conversions

pub type Unpacked = (usize, usize, u8, u8, u16, u16, Vec<f32>, Vec<f32>, Vec<u8>, Vec<u8>);

fn pack_scales(out: &mut Vec<u8>, scales: &[f32]) {
    out.extend_from_slice(&(scales.len() as u32).to_le_bytes());
    for s in scales {
        out.extend_from_slice(&s.to_le_bytes());
    }
}

// bounds-checked slice so truncated files give an error instead of a panic
fn take(bytes: &[u8], at: usize, n: usize) -> Result<&[u8], String> {
    at.checked_add(n)
        .and_then(|end| bytes.get(at..end))
        .ok_or_else(|| format!("truncated .face data: need {n} bytes at offset {at}, have {}", bytes.len()))
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(take(bytes, at, 4)?.try_into().unwrap()))
}

fn unpack_scales(bytes: &[u8], at: usize) -> Result<(Vec<f32>, usize), String> {
    let n = read_u32(bytes, at)? as usize;
    let start = at + 4;

    let scales = take(bytes, start, 4 * n)?
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect();

    Ok((scales, start + 4 * n))
}

fn pack_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn unpack_bytes(bytes: &[u8], at: usize) -> Result<(Vec<u8>, usize), String> {
    let n = read_u32(bytes, at)? as usize;
    let start = at + 4;

    Ok((take(bytes, start, n)?.to_vec(), start + n))
}

#[allow(clippy::too_many_arguments)]
pub fn pack(w: usize, h: usize, levels_s: u8, levels_d: u8, t0_s: u16, t0_d: u16, scales_s: &[f32], scales_d: &[f32], bytes_s: &[u8], bytes_d: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    out.extend_from_slice(&(w as u32).to_le_bytes());
    out.extend_from_slice(&(h as u32).to_le_bytes());

    out.push(levels_s);
    out.push(levels_d);

    // zerotree start thresholds, one per half-image
    out.extend_from_slice(&t0_s.to_le_bytes());
    out.extend_from_slice(&t0_d.to_le_bytes());

    pack_scales(&mut out, scales_s);
    pack_scales(&mut out, scales_d);

    pack_bytes(&mut out, bytes_s);
    pack_bytes(&mut out, bytes_d);

    out
}

pub fn unpack(bytes: &[u8]) -> Result<Unpacked, String> {
    let header = take(bytes, 0, 14)?;

    let w = u32::from_le_bytes(header[0..4].try_into().unwrap()) as usize;
    let h = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;

    let levels_s = header[8];
    let levels_d = header[9];

    let t0_s = u16::from_le_bytes(header[10..12].try_into().unwrap());
    let t0_d = u16::from_le_bytes(header[12..14].try_into().unwrap());

    let (scales_s, at) = unpack_scales(bytes, 14)?;
    let (scales_d, at) = unpack_scales(bytes, at)?;

    let (bytes_s, at) = unpack_bytes(bytes, at)?;
    let (bytes_d, _) = unpack_bytes(bytes, at)?;

    Ok((w, h, levels_s, levels_d, t0_s, t0_d, scales_s, scales_d, bytes_s, bytes_d))
}

#[test]
fn pack_unpack_roundtrip() {
    let scales_s = vec![5.0f32, 5.0, 5.0, 2.5];
    let scales_d = vec![50.0f32, 50.0, 50.0, 25.0, 12.5, 6.25, 3.125];
    let s = vec![1u8, 2, 3, 4, 5];
    let d = vec![9u8, 8];

    let packed = pack(128, 128, 1, 2, 64, 256, &scales_s, &scales_d, &s, &d);
    let (w, h, levels_s, levels_d, t0_s, t0_d, out_scales_s, out_scales_d, out_s, out_d) = unpack(&packed).unwrap();

    assert_eq!((w, h, levels_s, levels_d), (128, 128, 1, 2));
    assert_eq!((t0_s, t0_d), (64, 256));
    assert_eq!(out_scales_s, scales_s);
    assert_eq!(out_scales_d, scales_d);
    assert_eq!(out_s, s);
    assert_eq!(out_d, d);
}

#[test]
fn unpack_rejects_truncated_data() {
    let packed = pack(128, 128, 1, 2, 64, 256, &[5.0; 4], &[50.0; 7], &[1, 2, 3], &[9, 8]);

    for len in 0..packed.len() {
        assert!(unpack(&packed[..len]).is_err(), "len {len}");
    }
}
