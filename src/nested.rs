use crate::{Decoder, Encodable, Encoder, ErrorKind, Header, Length, Result, Slice, Tag, TaggedSlice};

/// Obtain the length of an ASN.1 `SEQUENCE` of [`Encodable`] values when
/// serialized as ASN.1 DER, including the `SEQUENCE` tag and length prefix.
pub fn encoded_length(/*tag: Tag,*/ encodables: &[&dyn Encodable]) -> Result<Length> {
    let inner_len = encoded_length_inner(encodables)?;
    // Header::new(tag, inner_len)?.encoded_length() + inner_len
    Header::new(crate::tag::MEANINGLESS_TAG, inner_len)?.encoded_length() + inner_len
}

/// Obtain the inner length of a container of [`Encodable`] values
/// excluding the tag and length.
pub(crate) fn encoded_length_inner(encodables: &[&dyn Encodable]) -> Result<Length> {
    encodables
        .iter()
        .fold(Ok(Length::zero()), |sum, encodable| {
            sum + encodable.encoded_length()?
        })
}

/// Nested BER-TLV data objects.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Nested<'a> {
    /// Tag
    pub(crate) tag: Tag,
    /// Inner value
    pub(crate) slice: Slice<'a>,
}

impl<'a> Nested<'a> {
    /// Create a new [`Nested`] from a slice
    pub fn new(tag: Tag, slice: &'a [u8]) -> Result<Self> {
        Slice::new(slice)
            .map(|slice| Self { tag, slice })
            .map_err(|_| ErrorKind::Length { tag }.into())
    }

    /// Borrow the inner byte sequence
    pub fn as_bytes(&self) -> &'a [u8] {
        self.slice.as_bytes()
    }


    /// Get Tag
    pub fn tag(&self) -> Tag {
        self.tag
    }

    /// Decode nested values, creating a new [`Decoder`] for
    /// the data contained in the sequence's body and passing it to the provided
    /// [`FnOnce`].
    pub fn decode_nested<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Decoder<'a>) -> Result<T>,
    {
        let mut nested_decoder = Decoder::new(self.as_bytes());
        let result = f(&mut nested_decoder)?;
        nested_decoder.finish(result)
    }
}

impl AsRef<[u8]> for Nested<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'a> From<TaggedSlice<'a>> for Nested<'a> {
    fn from(tagged_slice: TaggedSlice<'a>) -> Nested<'a> {
        Self { tag: tagged_slice.tag(), slice: tagged_slice.value }
    }
}

impl<'a> From<Nested<'a>> for TaggedSlice<'a> {
    fn from(nested: Nested<'a>) -> TaggedSlice<'a> {
        TaggedSlice { tag: nested.tag(), value: nested.slice }
    }
}

impl<'a> Encodable for Nested<'a> {
    fn encoded_length(&self) -> Result<Length> {
        TaggedSlice::from(*self).encoded_length()
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        TaggedSlice::from(*self).encode(encoder)
    }
}

