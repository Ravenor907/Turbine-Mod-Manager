//! MD5 (for Nexus's md5_search): the test suite of RFC 1321, appendix A.5, plus pieces fed in odd sizes and files.

use std::fs;
use turbine_engine::md5::{md5_file, Md5};
use turbine_engine::sha256::hex;

fn md5_hex(data: &[u8]) -> String {
    let mut m = Md5::new();
    m.update(data);
    hex(&m.finish())
}

const RFC_VECTORS: &[(&str, &str)] = &[
    ("", "d41d8cd98f00b204e9800998ecf8427e"),
    ("a", "0cc175b9c0f1b6a831c399e269772661"),
    ("abc", "900150983cd24fb0d6963f7d28e17f72"),
    ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
    ("abcdefghijklmnopqrstuvwxyz", "c3fcd3d76192e4007dfb496cca67e13b"),
    ("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789", "d174ab98d277d9f5a5611c2c9f419d9f"),
    (
        "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
        "57edf4a22be3c955ac49da2e2107b67a",
    ),
];

#[test]
fn rfc_1321_test_suite() {
    for (input, want) in RFC_VECTORS {
        assert_eq!(md5_hex(input.as_bytes()), *want, "MD5 of {input:?}");
    }
}

#[test]
fn pieces_of_any_size_give_the_same_hash() {
    let data: Vec<u8> = (0..1000u32).map(|i| (i * 31 % 251) as u8).collect();
    let whole = md5_hex(&data);
    for step in [1, 3, 55, 56, 63, 64, 65, 127, 999] {
        let mut m = Md5::new();
        for chunk in data.chunks(step) {
            m.update(chunk);
        }
        assert_eq!(hex(&m.finish()), whole, "fed {step} bytes at a time");
    }
    // Lengths around the padding edge (55, 56, 64 bytes) agree with feeding all at once.
    for len in [55usize, 56, 57, 63, 64, 65, 119, 120] {
        let d = vec![b'x'; len];
        let mut m = Md5::new();
        m.update(&d[..len / 2]);
        m.update(&d[len / 2..]);
        assert_eq!(hex(&m.finish()), md5_hex(&d));
    }
}

#[test]
fn files_are_hashed_in_pieces() {
    let p = std::env::temp_dir().join(format!("turbine-md5-{}", std::process::id()));
    fs::write(&p, b"message digest").unwrap();
    assert_eq!(md5_file(&p).unwrap(), "f96b697d7cb7938d525a2f31aaf161d0");
    let _ = fs::remove_file(&p);
    assert!(md5_file(&p).is_err());
}
