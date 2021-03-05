//! # flexiber
//!
//! Implementation of the BER-TLV serialization format from ISO 7816-4:2005.
//!
//! ITU-T X.690 (08/2015) defines the BER, CER and DER encoding rules for ASN.1
//!
//! The exact same document is [ISO/IET 8825-1][iso8825], which is freely available,
//! inconveniently packed as a single PDF in a ZIP file :)
//!
//! ## Credits
//! This library is a remix of `RustCrypto/utils/der`.
//!
//! The core idea taken from `der` is to have `Encodable` require an `encoded_length` method.
//! By calling this recursively in a first pass, allocations required in other approaches are
//! avoided.
//!
//! [iso8825]: https://standards.iso.org/ittf/PubliclyAvailableStandards/c068345_ISO_IEC_8825-1_2015.zip

#![no_std]
#![forbid(unsafe_code)]
// #![warn(missing_docs, rust_2018_idioms)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "derive")]
pub use flexiber_derive::{Decodable, Encodable};

#[cfg(feature = "std")]
extern crate std;

mod decoder;
mod encoder;
mod error;
mod header;
mod length;
mod simpletag;
mod slice;
mod tag;
mod tagged;
mod traits;

pub use decoder::Decoder;
pub use encoder::Encoder;
pub use error::{Error, ErrorKind, Result};
pub use length::Length;
pub use simpletag::SimpleTag;
pub use slice::Slice;
pub use tag::{Class, Tag, TagLike};
pub use tagged::{TaggedSlice, TaggedValue};
pub use traits::{Container, Decodable, Encodable, Tagged};
#[cfg(feature = "heapless")]
pub use traits::EncodableHeapless;

// #[derive(Clone, Copy, Debug, Decodable, Encodable, Eq, PartialEq)]
// struct T2<'a> {
//     #[tlv(simple = "0x55", slice)]
//     a: &'a [u8],
// }

