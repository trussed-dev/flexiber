//! BER-TLV headers.

use crate::{Decodable, Decoder, Encodable, Encoder, ErrorKind, Length, Result, TagLike};
use core::convert::TryInto;

/// BER-TLV headers: tag + length component of TLV-encoded values
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Header<T> {
    /// Tag representing the type of the encoded value
    pub tag: T,

    /// Length of the encoded value
    pub length: Length,
}

impl<T> Header<T> {
    /// Create a new [`Header`] from a [`Tag`] and a specified length.
    ///
    /// Returns [`Error`] if the length exceeds the limits of [`Length`]
    pub fn new(tag: T, length: impl TryInto<Length>) -> Result<Self> {
        let length = length.try_into().map_err(|_| ErrorKind::Overflow)?;
        Ok(Self { tag, length })
    }
}

impl<'a, T> Decodable<'a> for Header<T>
where
    T: Decodable<'a> + TagLike,
{
    fn decode<'b>(decoder: &'b mut Decoder<'a>) -> Result<Header<T>> {
        let tag = T::decode(decoder)?;

        let length = Length::decode(decoder).map_err(|e| {
            if e.kind() == ErrorKind::Overlength {
                ErrorKind::Length {
                    tag: tag.embedding(),
                }
                .into()
            } else {
                e
            }
        })?;

        Ok(Self { tag, length })
    }
}

impl<T> Encodable for Header<T>
where
    T: Encodable,
{
    fn encoded_length(&self) -> Result<Length> {
        self.tag.encoded_length()? + self.length.encoded_length()?
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        self.tag.encode(encoder)?;
        self.length.encode(encoder)
    }
}
