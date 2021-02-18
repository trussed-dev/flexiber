//! Tests for custom derive support

#![cfg(feature = "derive")]

use simple_tlv::{Decodable, Encodable};

#[derive(Clone, Copy, Debug, Decodable, Encodable, Eq, PartialEq)]
#[tlv(tag = "0xAA")]
struct S {
    #[tlv(slice, tag = "0x11")]
    x: [u8; 2],
    #[tlv(slice, tag = "0x22")]
    y: [u8; 3],
    #[tlv(slice, tag = "0x33")]
    z: [u8; 4],
}

#[derive(Clone, Copy, Debug, Decodable, Encodable, Eq, PartialEq)]
#[tlv(tag = "0xBB")]
struct T {
    #[tlv(tag = "0x44", slice)]
    x: [u8; 1234],
}

#[derive(Clone, Copy, Debug, Decodable, Encodable, Eq, PartialEq)]
struct T2 {
    #[tlv(tag = "0x44", slice)]
    x: [u8; 1234],
    #[tlv(tag = "0x55", slice)]
    a: [u8; 5],
}

#[test]
fn derived_reconstruct() {
    let s = S { x: [1,2], y: [3,4,5], z: [6,7,8,9] };
    let mut buf = [0u8; 1024];

    let encoded = s.encode_to_slice(&mut buf).unwrap();

    assert_eq!(encoded,
        &[0xAA, 15,
            0x11, 2, 1, 2,
            0x22, 3, 3, 4, 5,
            0x33, 4, 6, 7, 8, 9,
        ],
    );

    let s2 = S::from_bytes(encoded).unwrap();
    assert_eq!(s, s2);
}

#[test]
fn pretty_big() {
    let mut x = [0u8; 1234];
    for (i, x) in x.iter_mut().enumerate() {
        *x = i as _;
    };

    let t = T { x };

    let mut buf = [0u8; 1024];
    assert!(t.encode_to_slice(&mut buf).is_err());

    let mut buf = [0u8; 1500];
    let encoded = t.encode_to_slice(&mut buf).unwrap();

    assert_eq!(&encoded[..8], [
                    // 1234 + 4
        0xBB, 0xFF, 0x04, 0xD6,
                       // 1234
            0x44, 0xFF, 0x04, 0xD2]);
    assert_eq!(&encoded[8..], x);

    let t2 = T::from_bytes(encoded).unwrap();
    assert_eq!(t, t2);
}


#[test]
fn derive_untagged() {
    let mut x = [0u8; 1234];
    for (i, x) in x.iter_mut().enumerate() {
        *x = i as _;
    };

    let t = T2 { x, a: [17u8; 5] };

    let mut buf = [0u8; 1500];
    let encoded = t.encode_to_slice(&mut buf).unwrap();

    assert_eq!(&encoded[..4], [
                       // 1234
            0x44, 0xFF, 0x04, 0xD2]);
    assert_eq!(&encoded[4..(encoded.len() - 7)], x);
    assert_eq!(&encoded[(encoded.len() - 7)..], [0x55, 5, 17, 17, 17, 17, 17]);

    let t2 = T2::from_bytes(encoded).unwrap();
    assert_eq!(t, t2);
}
