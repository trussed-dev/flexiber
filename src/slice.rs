use core::convert::TryFrom;
use crate::{Length, Result};

/// Slice of at most `Length::max()` bytes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Slice<'a> {
    /// Inner value
    inner: &'a [u8],

    /// Precomputed `Length` (avoids possible panicking conversions)
    length: Length,
}

impl<'a> Slice<'a> {
    /// Create a new [`Slice`], ensuring that the provided `slice` value
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

    /// Get the [`Length`] of this [`Slice`]
    pub fn length(self) -> Length {
        self.length
    }

    /// Is this [`Slice`] empty?
    pub fn is_empty(self) -> bool {
        self.length() == Length::zero()
    }
}

impl AsRef<[u8]> for Slice<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

