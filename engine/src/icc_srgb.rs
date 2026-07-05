//! Minimal ICC v2.1 RGB matrix/TRC profile (sRGB, D50 PCS) for embedding as
//! the PDF/A `OutputIntent` `/DestOutputProfile`. Hand-built instead of
//! vendoring a third-party `.icc` file since PDF/A only requires a
//! structurally valid ICC profile that declares the sRGB primaries - the
//! spec doesn't need a specific vendor profile.
//!
//! ponytail: matrix/TRC profile with a single-gamma curve (2.2), not the
//! piecewise sRGB tone curve. Close enough for OutputIntent purposes (no
//! color management math actually reads this at render time); swap in a
//! real vendored sRGB.icc if a validator ever complains about curve shape.

/// D50-adapted sRGB primaries and white point, s15Fixed16 as (X, Y, Z) triples.
const WHITE_POINT: (f64, f64, f64) = (0.9642, 1.0000, 0.8249);
const RED_PRIMARY: (f64, f64, f64) = (0.4360, 0.2225, 0.0139);
const GREEN_PRIMARY: (f64, f64, f64) = (0.3851, 0.7169, 0.0971);
const BLUE_PRIMARY: (f64, f64, f64) = (0.1431, 0.0606, 0.7139);
const GAMMA: f64 = 2.2;

fn s15_fixed16(v: f64) -> [u8; 4] {
    ((v * 65536.0).round() as i32).to_be_bytes()
}

fn xyz_tag(xyz: (f64, f64, f64)) -> Vec<u8> {
    let mut data = Vec::with_capacity(20);
    data.extend_from_slice(b"XYZ ");
    data.extend_from_slice(&[0, 0, 0, 0]); // reserved
    data.extend_from_slice(&s15_fixed16(xyz.0));
    data.extend_from_slice(&s15_fixed16(xyz.1));
    data.extend_from_slice(&s15_fixed16(xyz.2));
    data
}

fn curve_tag_single_gamma(gamma: f64) -> Vec<u8> {
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(b"curv");
    data.extend_from_slice(&[0, 0, 0, 0]); // reserved
    data.extend_from_slice(&1u32.to_be_bytes()); // count = 1 -> single gamma entry
    let u8_fixed8 = (gamma * 256.0).round() as u16;
    data.extend_from_slice(&u8_fixed8.to_be_bytes());
    pad_to_4(&mut data);
    data
}

fn text_tag(s: &str) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + s.len() + 1);
    data.extend_from_slice(b"text");
    data.extend_from_slice(&[0, 0, 0, 0]); // reserved
    data.extend_from_slice(s.as_bytes());
    data.push(0);
    pad_to_4(&mut data);
    data
}

/// `textDescriptionType` ('desc'): ASCII description + empty Unicode/Mac
/// localizations, per ICC.1:2001-04 §6.5.17.
fn desc_tag(s: &str) -> Vec<u8> {
    let ascii_count = s.len() as u32 + 1; // + null terminator
    let mut data = Vec::new();
    data.extend_from_slice(b"desc");
    data.extend_from_slice(&[0, 0, 0, 0]); // reserved
    data.extend_from_slice(&ascii_count.to_be_bytes());
    data.extend_from_slice(s.as_bytes());
    data.push(0);
    data.extend_from_slice(&0u32.to_be_bytes()); // Unicode language code
    data.extend_from_slice(&0u32.to_be_bytes()); // Unicode description count (none)
    data.extend_from_slice(&0u16.to_be_bytes()); // Macintosh script code
    data.push(0); // Macintosh description count
    data.extend_from_slice(&[0u8; 67]); // Macintosh description (fixed 67 bytes)
    pad_to_4(&mut data);
    data
}

fn pad_to_4(data: &mut Vec<u8>) {
    while data.len() % 4 != 0 {
        data.push(0);
    }
}

/// Builds a complete minimal sRGB ICC profile suitable for a PDF/A
/// OutputIntent `/DestOutputProfile` (3-component RGB, `/N` = 3).
pub(crate) fn srgb_icc_profile() -> Vec<u8> {
    let tags: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"desc", desc_tag("sRGB IEC61966-2.1")),
        (*b"cprt", text_tag("Public Domain")),
        (*b"wtpt", xyz_tag(WHITE_POINT)),
        (*b"rXYZ", xyz_tag(RED_PRIMARY)),
        (*b"gXYZ", xyz_tag(GREEN_PRIMARY)),
        (*b"bXYZ", xyz_tag(BLUE_PRIMARY)),
        (*b"rTRC", curve_tag_single_gamma(GAMMA)),
        (*b"gTRC", curve_tag_single_gamma(GAMMA)),
        (*b"bTRC", curve_tag_single_gamma(GAMMA)),
    ];

    let tag_table_size = 4 + 12 * tags.len();
    let data_start = 128 + tag_table_size;

    let mut tag_table = Vec::with_capacity(tag_table_size);
    tag_table.extend_from_slice(&(tags.len() as u32).to_be_bytes());
    let mut tag_data = Vec::new();
    let mut offset = data_start;
    for (sig, data) in &tags {
        tag_table.extend_from_slice(sig);
        tag_table.extend_from_slice(&(offset as u32).to_be_bytes());
        tag_table.extend_from_slice(&(data.len() as u32).to_be_bytes());
        tag_data.extend_from_slice(data);
        offset += data.len();
    }

    let total_size = data_start + tag_data.len();

    let mut header = vec![0u8; 128];
    header[0..4].copy_from_slice(&(total_size as u32).to_be_bytes());
    // CMM type (4-7): left zero.
    header[8..12].copy_from_slice(&[2, 0x10, 0, 0]); // profile version 2.1.0
    header[12..16].copy_from_slice(b"mntr"); // device class: display device
    header[16..20].copy_from_slice(b"RGB "); // color space
    header[20..24].copy_from_slice(b"XYZ "); // PCS
                                             // datetime (24-35): left zero.
    header[36..40].copy_from_slice(b"acsp"); // profile file signature
                                             // platform / flags / manufacturer / model / attributes / rendering intent: left zero.
    header[68..80].copy_from_slice(&{
        let mut illum = Vec::with_capacity(12);
        illum.extend_from_slice(&s15_fixed16(WHITE_POINT.0));
        illum.extend_from_slice(&s15_fixed16(WHITE_POINT.1));
        illum.extend_from_slice(&s15_fixed16(WHITE_POINT.2));
        let arr: [u8; 12] = illum.try_into().unwrap();
        arr
    });
    // creator / reserved (80-127): left zero.

    let mut profile = header;
    profile.extend_from_slice(&tag_table);
    profile.extend_from_slice(&tag_data);
    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_has_valid_header() {
        let p = srgb_icc_profile();
        assert_eq!(&p[36..40], b"acsp", "missing ICC file signature");
        assert_eq!(&p[12..16], b"mntr");
        assert_eq!(&p[16..20], b"RGB ");
        assert_eq!(&p[20..24], b"XYZ ");
        let size = u32::from_be_bytes(p[0..4].try_into().unwrap()) as usize;
        assert_eq!(size, p.len(), "header size field must match actual length");
    }

    #[test]
    fn tag_table_offsets_are_in_bounds_and_4_byte_aligned() {
        let p = srgb_icc_profile();
        let count = u32::from_be_bytes(p[128..132].try_into().unwrap()) as usize;
        assert_eq!(count, 9);
        for i in 0..count {
            let entry = &p[132 + i * 12..132 + i * 12 + 12];
            let offset = u32::from_be_bytes(entry[4..8].try_into().unwrap()) as usize;
            let size = u32::from_be_bytes(entry[8..12].try_into().unwrap()) as usize;
            assert!(offset % 4 == 0, "tag offset must be 4-byte aligned");
            assert!(offset + size <= p.len(), "tag data must fit within profile");
        }
    }
}
