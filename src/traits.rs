// pub use der::{Decodable, Encodable};
//! Trait definitions

use core::convert::{TryFrom, TryInto};
use crate::{Decoder, Encoder, Error, header::Header, Length, Result, Tag, TaggedSlice, TaggedValue};

#[cfg(feature = "alloc")]
use {
    alloc::vec::Vec,
    core::iter,
    crate::ErrorKind,
};

#[cfg(feature = "heapless")]
use crate::ErrorKind;

/// Decoding trait.
///
/// Decode out of decoder, which essentially is a slice of bytes.
///
/// One way to implement this trait is to implement `TryFrom<TaggedSlice<'_>, Error = Error>`.
pub trait Decodable<'a>: Sized {
    /// Attempt to decode this message using the provided decoder.
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self>;

    /// Parse `Self` from the provided byte slice.
    fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        let mut decoder = Decoder::new(bytes);
        let result = Self::decode(&mut decoder)?;
        decoder.finish(result)
    }
}

impl<'a, T> Decodable<'a> for T
where
    T: TryFrom<TaggedSlice<'a>, Error = Error>,
{
    fn decode(decoder: &mut Decoder<'a>) -> Result<T> {
        TaggedSlice::decode(decoder)
            .and_then(Self::try_from)
            .or_else(|e| decoder.error(e.kind()))
    }
}

/// Encoding trait.
///
/// Encode into encoder, which essentially is a mutable slice of bytes.
///
/// Additionally, the encoded length needs to be known without actually encoding.
pub trait Encodable {
    /// Compute the length of this value in bytes when encoded as SIMPLE-TLV
    fn encoded_length(&self) -> Result<Length>;

    /// Encode this value as SIMPLE-TLV using the provided [`Encoder`].
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()>;

    /// Encode this value to the provided byte slice, returning a sub-slice
    /// containing the encoded message.
    fn encode_to_slice<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
        let mut encoder = Encoder::new(buf);
        self.encode(&mut encoder)?;
        Ok(encoder.finish()?)
    }

    /// Encode this message as SIMPLE-TLV, appending it to the provided
    /// byte vector.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn encode_to_vec(&self, buf: &mut Vec<u8>) -> Result<Length> {
        let expected_len = self.encoded_length()?.to_usize();
        let current_len = buf.len();
        buf.reserve(expected_len);
        buf.extend(iter::repeat(0).take(expected_len));

        // TODO(nickray): seems the original in `der` is incorrect here?
        // let mut encoder = Encoder::new(buf);
        let mut encoder = Encoder::new(&mut buf[current_len..]);
        self.encode(&mut encoder)?;
        let actual_len = encoder.finish()?.len();

        if expected_len != actual_len {
            return Err(ErrorKind::Underlength {
                expected: expected_len.try_into()?,
                actual: actual_len.try_into()?,
            }
            .into());
        }

        actual_len.try_into()
    }

    /// Serialize this message as a byte vector.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn to_vec(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode_to_vec(&mut buf)?;
        Ok(buf)
    }

}

#[cfg(feature = "heapless")]
#[cfg_attr(docsrs, doc(cfg(feature = "heapless")))]
/// The equivalent of the `encode_to_vec` and `to_vec` methods.
///
/// Separate trait because the generic parameter `N` would make `Encodable` not object safe.
pub trait EncodableHeapless: Encodable {
    /// Encode this message as SIMPLE-TLV, appending it to the provided
    /// heapless byte vector.
    fn encode_to_heapless_vec<N: heapless::ArrayLength<u8>>(&self, buf: &mut heapless::Vec<u8, N>) -> Result<Length> {
        let expected_len = self.encoded_length()?.to_usize();
        let current_len = buf.len();
        // TODO(nickray): add a specific error for "Overcapacity" conditional on heapless feature?
        buf.resize_default(current_len + expected_len).map_err(|_| Error::from(ErrorKind::Overlength))?;

        let mut encoder = Encoder::new(&mut buf[current_len..]);
        self.encode(&mut encoder)?;
        let actual_len = encoder.finish()?.len();

        if expected_len != actual_len {
            return Err(ErrorKind::Underlength {
                expected: expected_len.try_into()?,
                actual: actual_len.try_into()?,
            }
            .into());
        }

        actual_len.try_into()
    }

    /// Serialize this message as a byte vector.
    fn to_heapless_vec<N: heapless::ArrayLength<u8>>(&self) -> Result<heapless::Vec<u8, N>> {
        let mut buf = heapless::Vec::new();
        self.encode_to_heapless_vec(&mut buf)?;
        Ok(buf)
    }
}

/// Types that can be tagged.
pub(crate) trait Taggable: Sized {
    fn tagged(&self, tag: Tag) -> TaggedValue<&Self> {
        TaggedValue::new(tag, self)
    }
}

impl<X> Taggable for X where X: Sized {}

// /// Types with an associated SIMPLE-TLV [`Tag`].
// pub trait Tagged {
//     /// SIMPLE-TLV tag
//     const TAG: Tag;
// }

/// Types with an associated SIMPLE-TLV [`Tag`].
///
/// A tagged type implementing `Container` has a blanked implementation of `Encodable`.
pub trait Tagged {
    /// The tag
    fn tag() -> Tag;
}

/// Multiple encodables in a container.
///
/// A container implementing `Tagged` has a blanked implementation of `Encodable`.
pub trait Container {
    /// Call the provided function with a slice of [`Encodable`] trait objects
    /// representing the fields of this message.
    ///
    /// This method uses a callback because structs with fields which aren't
    /// directly [`Encodable`] may need to construct temporary values from
    /// their fields prior to encoding.
    fn fields<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&[&dyn Encodable]) -> Result<T>;
}

impl<TaggedContainer> Encodable for TaggedContainer
where
    TaggedContainer: Tagged + Container
{
    fn encoded_length(&self) -> Result<Length> {
        #[allow(clippy::redundant_closure)]
        // if we do as clippy tells, we get:
        // 183 |         let value_length = self.fields(Length::try_from)?;
        //     |                                 ^^^^^^ one type is more general than the other
        //     |
        //     = note: expected type `FnOnce<(&[&dyn Encodable],)>`
        //                found type `FnOnce<(&[&dyn Encodable],)>`
        let value_length = self.fields(|encodables| Length::try_from(encodables))?;
        Header::new(Self::tag(), value_length)?.encoded_length() + value_length
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        self.fields(|fields| encoder.encode_tagged_collection(Self::tag(), fields))
    }
}

///// Multiple encodables, nested under a SIMPLE-TLV tag.
/////
///// This wraps up a common pattern for SIMPLE-TLV encoding.
///// Implementations obtain a blanket `Encodable` implementation
//pub trait TaggedContainer: Container + Tagged {}

//pub trait Untagged {}

///// Multiple encodables, side-by-side without a SIMPLE-TLV tag.
/////
///// This wraps up a common pattern for SIMPLE-TLV encoding.
///// Implementations obtain a blanket `Encodable` implementation
//pub trait UntaggedContainer: Container + Untagged {}

// impl<UC> Encodable for UC
// where
//     UC: Untagged + Container,
// {
//     fn encoded_length(&self) -> Result<Length> {
//         todo!();
//         // let value_length = self.fields(|encodables| Length::try_from(encodables))?;
//         // Header::new(Self::tag(), value_length)?.encoded_length() + value_length
//     }

//     fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
//         todo!();
//         // self.fields(|fields| encoder.nested(Self::tag(), fields))
//     }
// }

// pub type UntaggedContainer<'a> = &'a [&'a dyn Encodable];

// impl<'a> Encodable for UntaggedContainer<'a> {
//     fn encoded_length(&self) -> Result<Length> {
//        Length::try_from(*self)
//     }

//     fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
//         for encodable in self.iter() {
//             encodable.encode(encoder)?;
//         }
//         Ok(())
//     }
// }

impl<'a> Encodable for &'a [u8] {
    fn encoded_length(&self) -> Result<Length> {
        self.len().try_into()
    }

    /// Encode this value as SIMPLE-TLV using the provided [`Encoder`].
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        encoder.bytes(self)
    }
}

macro_rules! impl_array {
    ($($N:literal),*) => {
        $(
            impl Encodable for [u8; $N] {
                fn encoded_length(&self) -> Result<Length> {
                    Ok(($N as u8).into())
                }

                /// Encode this value as SIMPLE-TLV using the provided [`Encoder`].
                fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
                    encoder.bytes(self.as_ref())
                }
            }

            impl Decodable<'_> for [u8; $N] {
                fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
                    use core::convert::TryInto;
                    let bytes: &[u8] = decoder.bytes($N as u8)?;
                    Ok(bytes.try_into().unwrap())
                }
            }
        )*
    }
}

impl_array!(
    0,1,2,3,4,5,6,7,8,9,
    10,11,12,13,14,15,16,17,18,19,
    20,21,22,23,24,25,26,27,28,29,
    30,31,32
);

#[cfg(test)]
mod tests {

    use core::convert::TryFrom;
    use crate::{Decodable, Encodable, Error, Result, Tag, TaggedSlice};
    use super::{Taggable, Tagged, Container};

    // The types [u8; 2], [u8; 3], [u8; 4] stand in here for any types for the fields
    // of a struct that are Decodable + Encodable. This means they can decode to/encode from
    // a byte slice, but also that thye can declare their encoded length.
    //
    // The goal then is to tag the struct definition for a proc-macro that implements
    // nested SIMPLE-TLV objects (as this is what we need in PIV return values)

    // tag 0xAA
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct S {
        // tag 0x11
        x: [u8; 2],
        // tag 0x22
        y: [u8; 3],
        // tag 0x33
        z: [u8; 4],
    }

    // this is what needs to be done to get `Decodable`
    impl<'a> TryFrom<TaggedSlice<'a>> for S {
        type Error = Error;

        fn try_from(tagged_slice: TaggedSlice<'a>) -> Result<S> {
            tagged_slice.tag().assert_eq(Tag::try_from(0xAA).unwrap())?;
            tagged_slice.decode_nested(|decoder| {
                let x = decoder.decode_tagged_value(Tag::try_from(0x11).unwrap())?;
                let y = decoder.decode_tagged_value(Tag::try_from(0x22).unwrap())?;
                let z = decoder.decode_tagged_value(Tag::try_from(0x33).unwrap())?;

                Ok(Self { x, y, z })
            })
        }
    }

    // this is what needs to be done to get `Encodable`
    impl Tagged for S {
        fn tag() -> Tag {
            Tag::try_from(0xAA).unwrap()
        }
    }

    impl Container for S {
        fn fields<F, T>(&self, field_encoder: F) -> Result<T>
        where
            F: FnOnce(&[&dyn Encodable]) -> Result<T>,
        {
            // both approaches equivalent
            field_encoder(&[
                &(Tag::try_from(0x11).unwrap().with_value(&self.x.as_ref())),
                // &self.x.tagged(Tag::try_from(0x11).unwrap()),
                &self.y.as_ref().tagged(Tag::try_from(0x22).unwrap()),
                &self.z.as_ref().tagged(Tag::try_from(0x33).unwrap()),

            ])
        }
    }

    #[test]
    fn reconstruct() {
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

    // tag 0xBB
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct T {
        // tag 0x01
        s: S,
        // tag 0x02
        t: [u8; 3],
    }

    impl<'a> TryFrom<TaggedSlice<'a>> for T {
        type Error = Error;

        fn try_from(tagged_slice: TaggedSlice<'a>) -> Result<Self> {
            tagged_slice.tag().assert_eq(Tag::try_from(0xBB).unwrap())?;
            tagged_slice.decode_nested(|decoder| {
                let s = decoder.decode_tagged_value(Tag::try_from(0x01).unwrap())?;
                let t = decoder.decode_tagged_value(Tag::try_from(0x02).unwrap())?;

                Ok(Self { s, t })
            })
        }
    }

    impl Tagged for T {
        fn tag() -> Tag {
            Tag::try_from(0xBB).unwrap()
        }
    }

    impl Container for T {
        fn fields<F, Z>(&self, field_encoder: F) -> Result<Z>
        where
            F: FnOnce(&[&dyn Encodable]) -> Result<Z>,
        {
            field_encoder(&[
                &self.s.tagged(Tag::try_from(0x1).unwrap()),
                &self.t.as_ref().tagged(Tag::try_from(0x2).unwrap()),
            ])
        }
    }


    #[test]
    fn nesty() {
        let s = S { x: [1,2], y: [3,4,5], z: [6,7,8,9] };
        let t = T { s, t: [0xA, 0xB, 0xC] };

        let mut buf = [0u8; 1024];

        let encoded = t.encode_to_slice(&mut buf).unwrap();

        assert_eq!(encoded,
            &[0xBB, 24,
                0x1, 17,
                    0xAA, 15,
                        0x11, 2, 1, 2,
                        0x22, 3, 3, 4, 5,
                        0x33, 4, 6, 7, 8, 9,
                0x2, 3,
                   0xA, 0xB, 0xC
            ],
        );

        let t2 = T::from_bytes(encoded).unwrap();

        assert_eq!(t, t2);
    }

    // tag 0xCC
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct T2 {
        // no tag
        s: S,
        // tag 0x02
        t: [u8; 3],
    }

    impl<'a> TryFrom<TaggedSlice<'a>> for T2 {
        type Error = Error;

        fn try_from(tagged_slice: TaggedSlice<'a>) -> Result<Self> {
            tagged_slice.tag().assert_eq(Tag::try_from(0xCC).unwrap())?;
            tagged_slice.decode_nested(|decoder| {
                let s = decoder.decode()?;
                let t = decoder.decode_tagged_value(Tag::try_from(0x02).unwrap())?;

                Ok(Self { s, t })
            })
        }
    }

    impl Tagged for T2 {
        fn tag() -> Tag {
            Tag::try_from(0xCC).unwrap()
        }
    }

    impl Container for T2 {
        fn fields<F, Z>(&self, field_encoder: F) -> Result<Z>
        where
            F: FnOnce(&[&dyn Encodable]) -> Result<Z>,
        {
            field_encoder(&[
                &self.s,
                &self.t.as_ref().tagged(Tag::try_from(0x2).unwrap()),
            ])
        }
    }


    #[test]
    fn nesty2() {
        let s = S { x: [1,2], y: [3,4,5], z: [6,7,8,9] };
        let t = T2 { s, t: [0xA, 0xB, 0xC] };

        let mut buf = [0u8; 1024];

        let encoded = t.encode_to_slice(&mut buf).unwrap();

        assert_eq!(encoded,
            // &[0xBB, 24,
            &[0xCC, 22,
                // 0x1, 17,
                    0xAA, 15,
                        0x11, 2, 1, 2,
                        0x22, 3, 3, 4, 5,
                        0x33, 4, 6, 7, 8, 9,
                0x2, 3,
                   0xA, 0xB, 0xC
            ],
        );

        let t2 = T2::from_bytes(encoded).unwrap();

        assert_eq!(t, t2);
    }

    // no tag
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct T3 {
        // no tag
        s: S,
        // tag 0x02
        t: [u8; 3],
    }

    // impl<'a> TryFrom<TaggedSlice<'a>> for T2 {
    //     type Error = Error;

    //     fn try_from(tagged_slice: TaggedSlice<'a>) -> Result<Self> {
    //         tagged_slice.tag().assert_eq(Tag::try_from(0xCC).unwrap())?;
    //         tagged_slice.decode_nested(|decoder| {
    //             let s = decoder.decode()?;
    //             let t = decoder.decode_tag(Tag::try_from(0x02).unwrap())?;

    //             Ok(Self { s, t })
    //         })
    //     }
    // }

    // impl TaggedContainer for T2 {
    //     fn tag() -> Tag {
    //         Tag::try_from(0xCC).unwrap()
    //     }

    //     fn fields<F, Z>(&self, field_encoder: F) -> Result<Z>
    //     where
    //         F: FnOnce(&[&dyn Encodable]) -> Result<Z>,
    //     {
    //         field_encoder(&[
    //             &self.s,
    //             &self.t.tagged(Tag::try_from(0x2).unwrap()),
    //         ])
    //     }
    // }


    // #[test]
    // fn nesty3() {
    //     let s = S { x: [1,2], y: [3,4,5], z: [6,7,8,9] };
    //     let t = T3 { s, t: [0xA, 0xB, 0xC] };

    //     let mut buf = [0u8; 1024];

    //     // let encoded = (&[
    //     //     &t.s,
    //     //     &t.t.tagged(Tag::try_from(0x2).unwrap()),
    //     // ]).encode_to_slice(&mut buf).unwrap();

    //     assert_eq!(encoded,
    //         // &[0xBB, 24,
    //         &[0xCC, 22,
    //             // 0x1, 17,
    //                 0xAA, 15,
    //                     0x11, 2, 1, 2,
    //                     0x22, 3, 3, 4, 5,
    //                     0x33, 4, 6, 7, 8, 9,
    //             0x2, 3,
    //                0xA, 0xB, 0xC
    //         ],
    //     );

    //     let t2 = T2::from_bytes(encoded).unwrap();

    //     assert_eq!(t, t2);
    // }
}
