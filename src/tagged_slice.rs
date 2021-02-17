// //! Common handling for types backed by byte slices with enforcement of the
// //! format-level length limitation of 65_535 bytes.

use crate::{Decodable, Decoder, Encodable, Encoder, ErrorKind, Header, Length, Nested, Result, Tag};
use core::convert::TryFrom;

/// Byte slice newtype which respects the `Length::max()` limit.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct ByteSlice<'a> {
    /// Inner value
    inner: &'a [u8],

    /// Precomputed `Length` (avoids possible panicking conversions)
    length: Length,
}

impl<'a> ByteSlice<'a> {
    /// Create a new [`ByteSlice`], ensuring that the provided `slice` value
    /// is shorter than `Length::max()`.
    pub fn new(slice: &'a [u8]) -> Result<Self> {
        Ok(Self {
            inner: slice,
            length: Length::try_from(slice.len())?,
        })
    }

    /// Borrow the inner byte slice
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner
    }

    /// Get the [`Length`] of this [`ByteSlice`]
    pub fn len(self) -> Length {
        self.length
    }

    /// Is this [`ByteSlice`] empty?
    pub fn is_empty(self) -> bool {
        self.len() == Length::zero()
    }
}

impl AsRef<[u8]> for ByteSlice<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

// pub fn TaggedAs(tag: Tag, data: &[u8]) -> impl Tagged<'_> {
//     struct

//     todo!();
// }

/// SIMPLE-TLV data object
///
/// TODO(nickray): rename to DataObject or similar (also module)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaggedSlice<'a> {
    pub(crate) tag: Tag,
    pub(crate) slice: ByteSlice<'a>,
}

// impl<'a, V: Decodable<'a> + Encodable> TaggedValue<'a, V> {
//     pub fn new(tag: Tag, value: &'a V) -> Self {
//         Self { tag, value }
//     }

//     pub fn tag(&self) -> Tag {
//         self.tag
//     }

//     // pub fn value(&self) -> &'a V {
//     // }

// }

impl<'a> TaggedSlice<'a> {
    /// Create a new tagged slice, checking lengths.
    pub fn new(tag: Tag, slice: &'a [u8]) -> Result<Self> {
        ByteSlice::new(slice)
            .map(|slice| Self { tag, slice })
            .map_err(|_| (ErrorKind::InvalidLength).into())
    }

    /// Borrow the inner byte slice.
    pub fn as_bytes(&self) -> &'a [u8] {
        self.slice.as_bytes()
    }

    pub fn tag(&self) -> Tag {
        self.tag
    }

    /// Get the length of the inner byte slice.
    pub fn len(&self) -> Length {
        self.slice.len()
    }

    /// Is the inner byte slice empty?
    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    /// Get the SIMPLE-TLV [`Header`] for this [`TaggedSlice`] value
    fn header(self) -> Result<Header> {
        Ok(Header {
            tag: self.tag,
            length: self.len(),
        })
    }

    /// Attempt to decode this value as nested TaggedSlices, creating a new
    /// nested [`Decoder`] and calling the provided argument with it.
    pub fn nested<F, T>(self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Decoder<'a>) -> Result<T>,
    {
        Nested::from(self).decode_nested(f)
    }
}

impl<'a> Decodable<'a> for TaggedSlice<'a> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<TaggedSlice<'a>> {
        let header = Header::decode(decoder)?;
        let tag = header.tag;
        let len = header.length.to_usize();
        let value = decoder.bytes(len).map_err(|_| ErrorKind::Length { tag })?;
        Self::new(tag, value)
    }
}

impl<'a> Encodable for TaggedSlice<'a> {
    fn encoded_len(&self) -> Result<Length> {
        self.header()?.encoded_len()? + self.len()
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        self.header()?.encode(encoder)?;
        encoder.bytes(self.as_bytes())
    }
}

// #[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub struct TaggedValue<V> {
//     pub(crate) tag: Tag,
//     pub(crate) value: PhantomData<V>,
// }

// impl<'a, V> Decodable<'a> for TaggedValue<'a, V>
// where
//     V: Decodable<'a> + Encodable,
// {
//     fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
//         let tagged_slice: TaggedSlice = decoder.decode()?;
//         // tagged_slice.tag().assert_eq(self.tag())?;
//         let value: &'a V = Decoder::new(tagged_slice.as_bytes()).decode()?;
//         Ok(Self { tag: tagged_slice.tag(), value })
//     }
// }

// impl<'a, V> Encodable for TaggedValue<'a, V>
// where
//     V: Decodable<'a> + Encodable,
// {
//     fn encoded_len(&self) -> Result<Length> {
//         self.header()?.encoded_len()? + self.len()
//     }

//     fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
//         self.header()?.encode(encoder)?;
//         encoder.bytes(self.value.as_bytes())
//     }
// }


#[cfg(test)]
mod tests {
    use core::convert::TryFrom;
    use crate::{Encodable, Tag, TaggedSlice};

    #[test]
    fn encode() {
        let mut buf = [0u8; 1024];

        let short = TaggedSlice::new(Tag::try_from(0x66).unwrap(), &[1, 2, 3]).unwrap();

        assert_eq!(
            short.encode_to_slice(&mut buf).unwrap(),
            &[0x66, 0x3, 1, 2, 3]
        );

        let slice = &[43u8; 256];
        let long = TaggedSlice::new(Tag::try_from(0x66).unwrap(), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..4], &[0x66, 0xFF, 0x01, 0x00]);
        assert_eq!(&encoded[4..], slice);
    }
}
