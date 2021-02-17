use core::convert::TryInto;
use crate::{Decodable, ErrorKind, Length, Result};

/// SIMPLE-TLV decoder.
#[derive(Debug)]
pub struct Decoder<'a> {
    /// Byte slice being decoded.
    ///
    /// In the event an error was previously encountered this will be set to
    /// `None` to prevent further decoding while in a bad state.
    bytes: Option<&'a [u8]>,

    /// Position within the decoded slice.
    position: Length,
}

impl<'a> Decoder<'a> {
    /// Create a new decoder for the given byte slice.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes: Some(bytes),
            position: Length::zero(),
        }
    }

    /// Decode a value which impls the [`Decodable`] trait.
    pub fn decode<T: Decodable<'a>>(&mut self) -> Result<T> {
        if self.is_failed() {
            self.error(ErrorKind::Failed)?;
        }

        T::decode(self).map_err(|e| {
            self.bytes.take();
            e.nested(self.position)
        })
    }

    pub fn decode_tag<T: Decodable<'a>>(&mut self, tag: crate::Tag) -> Result<T> {
        let tagged: crate::TaggedSlice = self.decode()?;
        tagged.tag().assert_eq(tag)?;
        Self::new(tagged.as_bytes()).decode()
    }

    /// Return an error with the given [`ErrorKind`], annotating it with
    /// context about where the error occurred.
    pub fn error<T>(&mut self, kind: ErrorKind) -> Result<T> {
        self.bytes.take();
        Err(kind.at(self.position))
    }

    /// Did the decoding operation fail due to an error?
    pub fn is_failed(&self) -> bool {
        self.bytes.is_none()
    }

    /// Finish decoding, returning the given value if there is no
    /// remaining data, or an error otherwise
    pub fn finish<T>(self, value: T) -> Result<T> {
        if self.is_failed() {
            Err(ErrorKind::Failed.at(self.position))
        } else if !self.is_finished() {
            Err(ErrorKind::TrailingData {
                decoded: self.position,
                remaining: self.remaining_len()?,
            }
            .at(self.position))
        } else {
            Ok(value)
        }
    }

    /// Have we decoded all of the bytes in this [`Decoder`]?
    ///
    /// Returns `false` if we're not finished decoding or if a fatal error
    /// has occurred.
    pub fn is_finished(&self) -> bool {
        self.remaining().map(|rem| rem.is_empty()).unwrap_or(false)
    }

    /// Decode a single byte, updating the internal cursor.
    pub(crate) fn byte(&mut self) -> Result<u8> {
        match self.bytes(1u8)? {
            [byte] => Ok(*byte),
            _ => self.error(ErrorKind::Truncated),
        }
    }

    /// Obtain a slice of bytes of the given length from the current cursor
    /// position, or return an error if we have insufficient data.
    pub(crate) fn bytes(&mut self, len: impl TryInto<Length>) -> Result<&'a [u8]> {
        if self.is_failed() {
            self.error(ErrorKind::Failed)?;
        }

        let len = len
            .try_into()
            .or_else(|_| self.error(ErrorKind::Overflow))?;

        let result = self
            .remaining()?
            .get(..len.to_usize())
            .ok_or(ErrorKind::Truncated)?;

        self.position = (self.position + len)?;
        Ok(result)
    }

    /// Obtain the remaining bytes in this decoder from the current cursor
    /// position.
    fn remaining(&self) -> Result<&'a [u8]> {
        self.bytes
            .and_then(|b| b.get(self.position.into()..))
            .ok_or_else(|| ErrorKind::Truncated.at(self.position))
    }

    /// Get the number of bytes still remaining in the buffer.
    fn remaining_len(&self) -> Result<Length> {
        self.remaining()?.len().try_into()
    }
}

impl<'a> From<&'a [u8]> for Decoder<'a> {
    fn from(bytes: &'a [u8]) -> Decoder<'a> {
        Decoder::new(bytes)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::Decoder;
//     use crate::{Decodable, ErrorKind, Length, Tag};

//     #[test]
//     fn truncated_message() {
//         let mut decoder = Decoder::new(&[]);
//         let err = bool::decode(&mut decoder).err().unwrap();
//         assert_eq!(ErrorKind::Truncated, err.kind());
//         assert_eq!(Some(Length::zero()), err.position());
//     }

//     #[test]
//     fn invalid_field_length() {
//         let mut decoder = Decoder::new(&[0x02, 0x01]);
//         let err = i8::decode(&mut decoder).err().unwrap();
//         assert_eq!(ErrorKind::Length { tag: Tag::Integer }, err.kind());
//         assert_eq!(Some(Length::from(2u8)), err.position());
//     }

//     #[test]
//     fn trailing_data() {
//         let mut decoder = Decoder::new(&[0x02, 0x01, 0x2A, 0x00]);
//         let x = decoder.decode().unwrap();
//         assert_eq!(42i8, x);

//         let err = decoder.finish(x).err().unwrap();
//         assert_eq!(
//             ErrorKind::TrailingData {
//                 decoded: 3u8.into(),
//                 remaining: 1u8.into()
//             },
//             err.kind()
//         );
//         assert_eq!(Some(Length::from(3u8)), err.position());
//     }
// }
