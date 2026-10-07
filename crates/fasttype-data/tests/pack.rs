use fasttype_data::pack::{Pack, PackError, PackWriter, compress, decompress_frame};

fn sample() -> Vec<u8> {
    let mut w = PackWriter::new();
    w.add("alpha", b"{\"words\":[\"a\"]}").unwrap();
    w.add("beta", "é".repeat(50_000).as_bytes()).unwrap();
    w.finish()
}

#[test]
fn roundtrip_keeps_names_and_bytes() {
    let bytes = sample();
    let pack = Pack::parse(&bytes).unwrap();
    let names: Vec<&str> = pack.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["alpha", "beta"]);
    assert_eq!(
        pack.decompress("alpha").unwrap().unwrap(),
        b"{\"words\":[\"a\"]}"
    );
    assert_eq!(
        pack.decompress("beta").unwrap().unwrap(),
        "é".repeat(50_000).as_bytes()
    );
    assert_eq!(pack.get("beta").unwrap().raw_len, 100_000);
    assert!(pack.get("beta").unwrap().len < 1_000, "compressé");
}

#[test]
fn unknown_entry_is_none() {
    let bytes = sample();
    assert!(
        Pack::parse(&bytes)
            .unwrap()
            .decompress("gamma")
            .unwrap()
            .is_none()
    );
}

#[test]
fn duplicate_names_are_refused() {
    let mut w = PackWriter::new();
    w.add("a", b"1").unwrap();
    assert!(w.add("a", b"2").is_err());
}

#[test]
fn bad_magic_is_rejected() {
    assert!(matches!(Pack::parse(b"nope"), Err(PackError::BadMagic)));
}

#[test]
fn truncated_pack_is_rejected() {
    let bytes = sample();
    for cut in [6, 9, 20, bytes.len() - 1] {
        assert!(
            matches!(Pack::parse(&bytes[..cut]), Err(PackError::Truncated)),
            "coupé à {cut}"
        );
    }
}

#[test]
fn corrupted_frame_is_an_error() {
    let mut bytes = sample();
    let n = bytes.len();
    for b in &mut bytes[n - 40..n - 10] {
        *b ^= 0xA5;
    }
    let pack = Pack::parse(&bytes).unwrap();
    assert!(pack.decompress("beta").is_err());
}

#[test]
fn frame_size_is_checked() {
    let frame = compress(b"hello").unwrap();
    assert_eq!(decompress_frame(&frame, 5).unwrap(), b"hello");
    assert!(matches!(
        decompress_frame(&frame, 6),
        Err(PackError::SizeMismatch {
            expected: 6,
            actual: 5
        })
    ));
}

#[test]
fn compression_is_deterministic() {
    assert_eq!(sample(), sample());
}

/// Réécrit le `raw_len` de chaque entrée dans l'index JSON d'un pack.
fn with_raw_len(bytes: &[u8], raw_len: u64) -> Vec<u8> {
    let index_len = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
    let mut index: serde_json::Value = serde_json::from_slice(&bytes[9..9 + index_len]).unwrap();
    for e in index.as_array_mut().unwrap() {
        e["raw_len"] = raw_len.into();
    }
    let index = serde_json::to_vec(&index).unwrap();
    let mut out = bytes[..5].to_vec();
    out.extend_from_slice(&(index.len() as u32).to_le_bytes());
    out.extend_from_slice(&index);
    out.extend_from_slice(&bytes[9 + index_len..]);
    out
}

#[test]
fn absurd_raw_len_in_index_is_an_error_not_a_panic() {
    let bytes = with_raw_len(&sample(), u64::MAX);
    match Pack::parse(&bytes) {
        Err(_) => {}
        Ok(pack) => assert!(pack.decompress("beta").is_err()),
    }
}

#[test]
fn frame_larger_than_announced_is_rejected_without_reading_it_all() {
    let frame = compress(&vec![b'a'; 1_000_000]).unwrap();
    assert!(matches!(
        decompress_frame(&frame, 10),
        Err(PackError::SizeMismatch { expected: 10, .. })
    ));
}
