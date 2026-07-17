//! InputOtp builder struct — one-time passcode digit input.

/// An OTP input: individual digit boxes in a row.
#[must_use]
pub struct InputOtp {
    pub(crate) length: usize,
}

impl InputOtp {
    pub fn new(length: usize) -> Self {
        Self {
            length: length.max(1),
        }
    }
}
