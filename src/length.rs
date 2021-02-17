//! Length calculations for encoded ASN.1 DER values

use crate::{Decodable, Decoder, Encodable, Encoder, Error, ErrorKind, Result};
use core::{convert::{TryFrom, TryInto}, fmt, ops::Add};

/// SIMPLE-TLV-encoded length.
///
/// By definition, in the range `0..=65535`
///
/// The length field consists of one or three consecutive bytes.
/// - If the first byte is not set to 'FF', then the length field consists of a single byte encoding a number from
///   zero to 254 and denoted N.
/// - If the first byte is set to 'FF', then the length field continues on the subsequent two bytes with any
///   value encoding a number from zero to 65_535 and denoted N.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct Length(u16);

impl Length {
    /// Return a length of `0`.
    pub const fn zero() -> Self {
        Length(0)
    }

    /// Get the maximum length supported by this crate
    pub const fn max() -> usize {
        u16::MAX as usize
    }

    /// Convert length to `usize`
    pub fn to_usize(self) -> usize {
        self.0.into()
    }
}

impl Add for Length {
    type Output = Result<Self>;

    fn add(self, other: Self) -> Result<Self> {
        self.0
            .checked_add(other.0)
            .map(Length)
            .ok_or_else(|| ErrorKind::Overflow.into())
    }
}

impl Add<u8> for Length {
    type Output = Result<Self>;

    fn add(self, other: u8) -> Result<Self> {
        self + Length::from(other)
    }
}

impl Add<u16> for Length {
    type Output = Result<Self>;

    fn add(self, other: u16) -> Result<Self> {
        self + Length::from(other)
    }
}

impl Add<usize> for Length {
    type Output = Result<Self>;

    fn add(self, other: usize) -> Result<Self> {
        self + Length::try_from(other)?
    }
}

impl Add<Length> for Result<Length> {
    type Output = Self;

    fn add(self, other: Length) -> Self {
        self? + other
    }
}

impl From<u8> for Length {
    fn from(len: u8) -> Length {
        Length(len as u16)
    }
}

impl From<u16> for Length {
    fn from(len: u16) -> Length {
        Length(len)
    }
}

impl From<Length> for u16 {
    fn from(len: Length) -> u16 {
        len.0
    }
}

impl From<Length> for usize {
    fn from(len: Length) -> usize {
        len.0 as usize
    }
}

impl TryFrom<usize> for Length {
    type Error = Error;

    fn try_from(len: usize) -> Result<Length> {
        u16::try_from(len)
            .map(Length)
            .map_err(|_| ErrorKind::Overflow.into())
    }
}

impl Decodable<'_> for Length {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Length> {
        match decoder.byte()? {
            0xFF => {
                let be_len = decoder.bytes(2u8)?;
                Ok(Length::from(u16::from_be_bytes(be_len.try_into().unwrap())))
            }
            len => Ok(len.into()),
        }
    }
}

impl Encodable for Length {
    fn encoded_len(&self) -> Result<Length> {
        match self.0 {
            0..=0xFE => Ok(Length(1)),
            _ => Ok(Length(3)),
        }
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        match self.0 {
            0..=0xFE => encoder.byte(self.0 as u8),
            _ => {
                encoder.byte(0xFF)?;
                encoder.bytes(&self.0.to_be_bytes())
            }
        }
    }
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::Length;
    use crate::{Decodable, Encodable, Error, ErrorKind};

    #[test]
    fn decode() {
        assert_eq!(Length::zero(), Length::from_bytes(&[0x00]).unwrap());

        assert_eq!(Length::from(0x7Fu8), Length::from_bytes(&[0x7F]).unwrap());
        assert_eq!(Length::from(0x7Fu8), Length::from_bytes(&[0xFF, 0x00, 0x7F]).unwrap());
        assert_eq!(Length::from(0xFEu8), Length::from_bytes(&[0xFE]).unwrap());
        assert_eq!(Length::from(0xFEu8), Length::from_bytes(&[0xFF, 0x00, 0xFE]).unwrap());

        // these are the current errors, do we want them?
        assert_eq!(Length::from_bytes(&[0xFF]).unwrap_err(), Error::from(ErrorKind::Truncated));
        assert_eq!(Length::from_bytes(&[0xFF, 0x12]).unwrap_err(), Error::from(ErrorKind::Truncated));
        // this is a bit clumsy to express
        assert!(Length::from_bytes(&[0xFF, 0x12, 0x34, 0x56]).is_err());


        assert_eq!(
            Length::from(0xFFu8),
            Length::from_bytes(&[0xFF, 0x00, 0xFF]).unwrap()
        );

        assert_eq!(
            Length::from(0x100u16),
            Length::from_bytes(&[0xFF, 0x01, 0x00]).unwrap()
        );

        assert_eq!(
            Length::from(0xFFFFu16),
            Length::from_bytes(&[0xFF, 0xFF, 0xFF]).unwrap()
        );
    }

    #[test]
    fn encode() {
        let mut buffer = [0u8; 3];

        assert_eq!(
            &[0x00],
            Length::zero().encode_to_slice(&mut buffer).unwrap()
        );

        assert_eq!(
            &[0x7F],
            Length::from(0x7Fu8).encode_to_slice(&mut buffer).unwrap()
        );

        assert_eq!(
            &[0xFE],
            Length::from(0xFEu8).encode_to_slice(&mut buffer).unwrap()
        );

        assert_eq!(
            &[0xFF, 0x00, 0xFF],
            Length::from(0xFFu8).encode_to_slice(&mut buffer).unwrap()
        );

        assert_eq!(
            &[0xFF, 0x01, 0x00],
            Length::from(0x100u16).encode_to_slice(&mut buffer).unwrap()
        );

        assert_eq!(
            &[0xFF, 0xFF, 0xFF],
            Length::from(0xFFFFu16).encode_to_slice(&mut buffer).unwrap()
        );
    }
}
