use crate::{Decoder, Encodable, Encoder, ErrorKind, Header, Length, Result, Tag, tagged_slice::ByteSlice, TaggedSlice};

/// Obtain the length of an ASN.1 `SEQUENCE` of [`Encodable`] values when
/// serialized as ASN.1 DER, including the `SEQUENCE` tag and length prefix.
pub fn encoded_len(/*tag: Tag,*/ encodables: &[&dyn Encodable]) -> Result<Length> {
    let inner_len = encoded_len_inner(encodables)?;
    // Header::new(tag, inner_len)?.encoded_len() + inner_len
    Header::new(crate::tag::MEANINGLESS_TAG, inner_len)?.encoded_len() + inner_len
}

/// Obtain the inner length of a container of [`Encodable`] values
/// excluding the tag and length.
pub(crate) fn encoded_len_inner(encodables: &[&dyn Encodable]) -> Result<Length> {
    encodables
        .iter()
        .fold(Ok(Length::zero()), |sum, encodable| {
            sum + encodable.encoded_len()?
        })
}

/// Nested SIMPLE-TLV data objects.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Nested<'a> {
    /// Tag
    pub(crate) tag: Tag,
    /// Inner value
    pub(crate) slice: ByteSlice<'a>,
}

impl<'a> Nested<'a> {
    /// Create a new [`Nested`] from a slice
    pub fn new(tag: Tag, slice: &'a [u8]) -> Result<Self> {
        ByteSlice::new(slice)
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
        Self { tag: tagged_slice.tag(), slice: tagged_slice.slice }
    }
}

impl<'a> From<Nested<'a>> for TaggedSlice<'a> {
    fn from(nested: Nested<'a>) -> TaggedSlice<'a> {
        TaggedSlice { tag: nested.tag(), slice: nested.slice }
    }
}

impl<'a> Encodable for Nested<'a> {
    fn encoded_len(&self) -> Result<Length> {
        TaggedSlice::from(*self).encoded_len()
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        TaggedSlice::from(*self).encode(encoder)
    }
}

