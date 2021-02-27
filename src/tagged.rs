// //! Common handling for types backed by byte slices with enforcement of the
// //! format-level length limitation of 65,535 bytes.

use crate::{Decodable, Decoder, Encodable, Encoder, ErrorKind, header::Header, Length, Result, Slice, Tag, TagLike};

/// BER-TLV data object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaggedValue<V, T=Tag> {
    tag: T,
    value: V,
}

/// Raw BER-TLV data object `TaggedValue<Slice<'_>>`.
pub type TaggedSlice<'a, T=Tag> = TaggedValue<Slice<'a>, T>;

impl<V, T> TaggedValue<V, T>
where
    T: Copy,
{
    pub fn new(tag: T, value: V) -> Self {
        Self { tag, value }
    }

    pub fn tag(&self) -> T {
        self.tag
    }
}

impl<'a, E, T> TaggedValue<&'a E, T>
where
    E: Encodable,
    T: Copy + Encodable,
{
    fn header(&self) -> Result<Header<T>> {
        Ok(Header {
            tag: self.tag(),
            length: self.value.encoded_length()?,
        })
    }
}

impl<'a, E, T> Encodable for TaggedValue<&'a E, T>
where
    E: Encodable,
    T: Copy + Encodable,
{
    fn encoded_length(&self) -> Result<Length> {
        self.header()?.encoded_length()? + self.value.encoded_length()?
    }
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        self.header()?.encode(encoder)?;
        encoder.encode(self.value)
    }
}

impl<'a, T> TaggedSlice<'a, T>
where
    T: Copy
{

    /// Create a new tagged slice, checking lengths.
    pub fn from(tag: T, slice: &'a [u8]) -> Result<Self> {
        Slice::new(slice)
            .map(|slice| Self { tag, value: slice })
            .map_err(|_| (ErrorKind::InvalidLength).into())
    }

    /// Borrow the inner byte slice.
    pub fn as_bytes(&self) -> &'a [u8] {
        self.value.as_bytes()
    }

    /// Get the length of the inner byte slice.
    pub fn length(&self) -> Length {
        self.value.length()
    }

    /// Is the inner byte slice empty?
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Get the BER-TLV [`Header`] for this [`TaggedSlice`] value
    #[allow(clippy::unnecessary_wraps)]
    fn header(&self) -> Result<Header<T>> {
        Ok(Header {
            tag: self.tag(),
            length: self.length(),
        })
    }

    /// Decode nested values, creating a new [`Decoder`] for
    /// the data contained in the sequence's body and passing it to the provided
    /// [`FnOnce`].
    pub fn decode_nested<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Decoder<'a>) -> Result<R>,
    {
        let mut nested_decoder = Decoder::new(self.as_bytes());
        let result = f(&mut nested_decoder)?;
        nested_decoder.finish(result)
    }
}

impl<'a, T> Decodable<'a> for TaggedSlice<'a, T>
where
    T: Decodable<'a> + TagLike,
{
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
        let header = Header::<T>::decode(decoder)?;
        let tag = header.tag;
        let len = header.length.to_usize();
        let value = decoder.bytes(len).map_err(|_| ErrorKind::Length { tag: tag.embedding() })?;
        Self::from(tag, value)
    }
}

impl<'a, T> Encodable for TaggedSlice<'a, T>
where
    T: Copy + Encodable
{
    fn encoded_length(&self) -> Result<Length> {
        self.header()?.encoded_length()? + self.length()
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        self.header()?.encode(encoder)?;
        encoder.bytes(self.as_bytes())
    }
}

// /// Obtain the length of an ASN.1 `SEQUENCE` of [`Encodable`] values when
// /// serialized as ASN.1 DER, including the `SEQUENCE` tag and length prefix.
// pub fn encoded_length2(/*tag: Tag,*/ encodables: &[&dyn Encodable]) -> Result<Length> {
//     let inner_len = Length::try_from(encodables)?;
//     Header::new(crate::tag::MEANINGLESS_TAG, inner_len)?.encoded_length() + inner_len
// }

// /// Obtain the inner length of a container of [`Encodable`] values
// /// excluding the tag and length.
// pub(crate) fn sum_encoded_lengths(encodables: &[&dyn Encodable]) -> Result<Length> {
//     encodables
//         .iter()
//         .fold(Ok(Length::zero()), |sum, encodable| {
//             sum + encodable.encoded_length()?
//         })
// }


#[cfg(test)]
mod tests {
    use core::convert::TryFrom;
    use crate::{Encodable, Tag, TaggedSlice};

    #[test]
    fn encode() {
        let mut buf = [0u8; 1024];

        let short = TaggedSlice::from(Tag::try_from(0x06).unwrap(), &[1, 2, 3]).unwrap();

        assert_eq!(
            short.encode_to_slice(&mut buf).unwrap(),
            &[0x06, 0x3, 1, 2, 3]
        );

        let slice = &[43u8; 256];

        let long = TaggedSlice::from(Tag::universal(0x66), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..5], &[0x1F, 0x66, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[5..], slice);

        let long = TaggedSlice::from(Tag::universal(0x66).constructed(), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..5], &[0x3F, 0x66, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[5..], slice);

        let long = TaggedSlice::from(Tag::application(0x66), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..5], &[0x5F, 0x66, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[5..], slice);

        let long = TaggedSlice::from(Tag::application(0x66).constructed(), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..5], &[0x7F, 0x66, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[5..], slice);

        let long = TaggedSlice::from(Tag::context(0x66), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..5], &[0x9F, 0x66, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[5..], slice);

        let long = TaggedSlice::from(Tag::context(0x66).constructed(), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..5], &[0xBF, 0x66, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[5..], slice);

        let long = TaggedSlice::from(Tag::private(0x66), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..5], &[0xDF, 0x66, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[5..], slice);

        let long = TaggedSlice::from(Tag::private(0x66).constructed(), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..5], &[0xFF, 0x66, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[5..], slice);

    }

}
