//! # simple-tlv
//!
//! From ISO 7816-4:2005:
//! ### 5.2.1 SIMPLE-TLV data objects
//! Each SIMPLE-TLV data object shall consist of two or three consecutive fields: a mandatory tag field, a
//! mandatory length field and a conditional value field. A record (see 7.3.1) may be a SIMPLE-TLV data object.
//! - The tag field consists of a single byte encoding a tag number from 1 to 254. The values '00' and 'FF' are
//!   invalid for tag fields. If a record is a SIMPLE-TLV data object, then the tag may be used as record identifier.
//! - The length field consists of one or three consecutive bytes.
//!   - If the first byte is not set to 'FF', then the length field consists of a single byte encoding a number from
//!     zero to 254 and denoted N.
//!   - If the first byte is set to 'FF', then the length field continues on the subsequent two bytes with any
//!     value encoding a number from zero to 65_535 and denoted N.
//! - If N is zero, there is no value field, i.e., the data object is empty. Otherwise (N > 0), the value field
//!   consists of N consecutive bytes.
//!
//! ## Origin
//! This library is obviously a remix of `RustCrypto/utils/der`, with a view towards:
//! - not requiring references to ASN.1 (e.g., since SIMPLE-TLV does not have any)
//! - not requiring allocations or memmoves (like ring, derp, x509:der)
//! - adding a type layer on top of SIMPLE-TLV's byte slice values

#![no_std]
#![forbid(unsafe_code)]
// #![warn(missing_docs, rust_2018_idioms)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod decoder;
mod encoder;
mod error;
mod header;
mod length;
mod nested;
mod tag;
mod tagged_slice;
mod traits;

pub use tagged_slice::TaggedSlice;
pub use decoder::Decoder;
pub use encoder::Encoder;
pub use error::{Error, ErrorKind, Result};
pub(crate) use header::Header;
pub use length::Length;
pub use nested::Nested;
pub use tag::Tag;
pub use traits::{Decodable, Encodable, Tagged};

