//! Tests for custom derive support

#![cfg(feature = "derive")]

use flexiber as ber;
use flexiber::{Decodable, Encodable};

#[derive(Clone, Copy, Debug, Decodable, Encodable, Eq, PartialEq)]
#[tlv(number = "0xAA")]
struct S {
    #[tlv(slice, number = "0x11")]
    x: [u8; 2],
    #[tlv(slice, number = "0x22")]
    y: [u8; 3],
    #[tlv(slice, number = "0x33")]
    z: [u8; 4],
}

#[derive(Clone, Copy, Debug, Decodable, Encodable, Eq, PartialEq)]
#[tlv(application, number = "0xAA")]
struct SApp {
    #[tlv(slice, number = "0x11")]
    x: [u8; 2],
    #[tlv(slice, number = "0x22")]
    y: [u8; 3],
    #[tlv(slice, number = "0x33")]
    z: [u8; 4],
}

#[test]
fn derived_reconstruct() {
    let s = S {
        x: [1, 2],
        y: [3, 4, 5],
        z: [6, 7, 8, 9],
    };
    let mut buf = [0u8; 1024];

    let encoded = s.encode_to_slice(&mut buf).unwrap();

    assert_eq!(
        encoded,
        &[0x1F, 0x81, 0x2A, 17, 0x11, 2, 1, 2, 0x1F, 0x22, 3, 3, 4, 5, 0x1F, 0x33, 4, 6, 7, 8, 9,],
    );

    let s2 = S::from_bytes(encoded).unwrap();
    assert_eq!(s, s2);
}

#[test]
fn derived_reconstruct_application() {
    let s = SApp {
        x: [1, 2],
        y: [3, 4, 5],
        z: [6, 7, 8, 9],
    };
    let mut buf = [0u8; 1024];

    let encoded = s.encode_to_slice(&mut buf).unwrap();

    assert_eq!(
        encoded,
        &[0x5F, 0x81, 0x2A, 17, 0x11, 2, 1, 2, 0x1F, 0x22, 3, 3, 4, 5, 0x1F, 0x33, 4, 6, 7, 8, 9,],
    );

    let s2 = SApp::from_bytes(encoded).unwrap();
    assert_eq!(s, s2);
}

#[derive(Clone, Copy, Debug, Decodable, Encodable, Eq, PartialEq)]
#[tlv(constructed, number = "0x10")]
struct T {
    #[tlv(number = "0x44", slice)]
    x: [u8; 1234],
}

#[test]
fn pretty_big() {
    let mut x = [0u8; 1234];
    for (i, x) in x.iter_mut().enumerate() {
        *x = i as _;
    }

    let t = T { x };

    let mut buf = [0u8; 1024];
    assert!(t.encode_to_slice(&mut buf).is_err());

    let mut buf = [0u8; 1500];
    let encoded = t.encode_to_slice(&mut buf).unwrap();

    assert_eq!(
        &encoded[..9],
        [
            // 1234 + 5
            0x30,
            0x82,
            0x04,
            0xD2 + 5,
            // 1234
            0x1F,
            0x44,
            0x82,
            0x04,
            0xD2
        ]
    );
    assert_eq!(&encoded[9..], x);

    let t2 = T::from_bytes(encoded).unwrap();
    assert_eq!(t, t2);
}

#[derive(Clone, Copy, Debug, Decodable, Encodable, Eq, PartialEq)]
struct T2 {
    #[tlv(private, primitive, number = "0x44", slice)]
    x: [u8; 1234],
    #[tlv(simple = "0x55", slice)]
    a: [u8; 5],
}

#[test]
fn derive_untagged() {
    let mut x = [0u8; 1234];
    for (i, x) in x.iter_mut().enumerate() {
        *x = i as _;
    }

    let t = T2 { x, a: [17u8; 5] };

    let mut buf = [0u8; 1500];
    let encoded = t.encode_to_slice(&mut buf).unwrap();

    assert_eq!(
        &encoded[..5],
        [
            // 1234
            223, 0x44, 0x82, 0x04, 0xD2
        ]
    );
    assert_eq!(&encoded[5..(encoded.len() - 7)], x);
    assert_eq!(
        &encoded[(encoded.len() - 7)..],
        [0x55, 5, 17, 17, 17, 17, 17]
    );

    let t2 = T2::from_bytes(encoded).unwrap();
    assert_eq!(t, t2);
}

#[derive(Clone, Copy)]
pub struct PinUsagePolicy {
    piv_pin: bool,
    global_pin: bool,
    on_card_biometric_comparison: bool,

    has_virtual_contact_interface: bool,
    pairing_code_required_for_vci: Option<bool>,

    cardholder_prefers_global_pin: Option<bool>,
}

impl Default for PinUsagePolicy {
    fn default() -> Self {
        Self {
            piv_pin: true,
            global_pin: false,
            on_card_biometric_comparison: false,
            has_virtual_contact_interface: false,
            pairing_code_required_for_vci: None,
            cardholder_prefers_global_pin: None,
        }
    }
}

impl Decodable<'_> for PinUsagePolicy {
    fn decode(decoder: &mut ber::Decoder<'_>) -> ber::Result<Self> {
        let raw: [u8; 2] = decoder.decode()?;
        let capabilities = raw[0];
        let has_global_pin = capabilities & (1 << 5) != 0;
        let has_virtual_contact_interface = capabilities & (1 << 3) != 0;
        Ok(Self {
            piv_pin: capabilities & (1 << 6) != 0,
            global_pin: has_global_pin,
            on_card_biometric_comparison: capabilities & (1 << 4) != 0,
            has_virtual_contact_interface,
            pairing_code_required_for_vci: if has_virtual_contact_interface {
                Some(capabilities & (1 << 2) != 0)
            } else {
                None
            },

            cardholder_prefers_global_pin: if has_global_pin {
                Some(raw[1] == 0x20)
            } else {
                None
            },
        })
    }
}

impl Encodable for PinUsagePolicy {
    fn encoded_length(&self) -> ber::Result<ber::Length> {
        Ok(2u8.into())
    }

    fn encode(&self, encoder: &mut ber::Encoder<'_>) -> ber::Result<()> {
        let mut first_byte = 0u8;
        if self.piv_pin {
            first_byte |= 1 << 6;
        }
        if self.global_pin {
            first_byte |= 1 << 5;
        }
        if self.on_card_biometric_comparison {
            first_byte |= 1 << 4;
        }
        if self.has_virtual_contact_interface {
            first_byte |= 1 << 3;
        }

        if self.has_virtual_contact_interface && Some(true) == self.pairing_code_required_for_vci {
            first_byte |= 1 << 2;
        }

        let mut second_byte = 0u8;
        if self.global_pin {
            if let Some(prefers_global) = self.cardholder_prefers_global_pin {
                if prefers_global {
                    second_byte = 0x20;
                } else {
                    second_byte = 0x10;
                }
            }
        }

        encoder.encode(&[first_byte, second_byte])
    }
}

#[derive(Decodable, Encodable)]
#[tlv(application, constructed, number = "0x1E")] // = 0x7E
pub struct DiscoveryObject {
    #[tlv(slice, application, number = "0xF")]
    piv_card_application_aid: [u8; 11],
    #[tlv(application, number = "0x2f")]
    pin_usage_policy: PinUsagePolicy,
}

impl Default for DiscoveryObject {
    fn default() -> Self {
        Self {
            piv_card_application_aid: hex_literal::hex!("A000000308 00001000 0100"),
            pin_usage_policy: Default::default(), //[0x40, 0x00],
        }
    }
}

#[test]
fn discovery() {
    let disco = DiscoveryObject::default();
    let mut buf = [0u8; 64];
    let encoded = disco.encode_to_slice(&mut buf).unwrap();
    assert_eq!(
        encoded,
        hex_literal::hex!("7e124f0ba0000003080000100001005f2f024000")
    );
}
