use core::fmt;

/// A canonical 256-bit identity supplied by an authenticated integration.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity([u8; 32]);

impl Identity {
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn is_present(&self) -> bool {
        self.0.iter().any(|byte| *byte != 0)
    }
}

impl fmt::Debug for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Identity")
            .field(&"<redacted>")
            .finish()
    }
}

/// A generational request identity. Reusing a slot changes its generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId {
    slot: u32,
    generation: u32,
}

impl RequestId {
    #[must_use]
    pub const fn new(slot: u32, generation: u32) -> Self {
        Self { slot, generation }
    }

    #[must_use]
    pub const fn slot(self) -> u32 {
        self.slot
    }

    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::Identity;

    #[test]
    fn all_zero_identity_is_absent() {
        assert!(!Identity::new([0; 32]).is_present());

        let mut bytes = [0; 32];
        bytes[31] = 1;
        assert!(Identity::new(bytes).is_present());
    }
}
