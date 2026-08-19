use core::fmt;
use vstd::prelude::*;

verus! {

/// A canonical 256-bit identity supplied by an authenticated integration.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity([u8; 32]);

impl Identity {
    pub closed spec fn bytes_spec(&self) -> Seq<u8> {
        self.0@
    }

    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> (identity: Self)
        ensures identity.bytes_spec() == bytes@,
    {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> (bytes: &[u8; 32])
        ensures bytes@ == self.bytes_spec(),
    {
        &self.0
    }

    #[must_use]
    pub fn is_present(&self) -> (present: bool)
        ensures
            present == exists|index: int|
                0 <= index < self.bytes_spec().len()
                    && self.bytes_spec()[index] != 0,
    {
        let mut index = 0;
        while index < self.0.len()
            invariant
                0 <= index <= self.bytes_spec().len(),
                forall|prior: int|
                    0 <= prior < index ==> self.bytes_spec()[prior] == 0,
            decreases self.bytes_spec().len() - index,
        {
            if self.0[index] != 0 {
                return true;
            }
            index += 1;
        }
        false
    }
}

} // verus!

impl fmt::Debug for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Identity")
            .field(&"<redacted>")
            .finish()
    }
}

verus! {

/// A generational request identity. Reusing a slot changes its generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId {
    slot: u32,
    generation: u32,
}

impl RequestId {
    pub closed spec fn slot_spec(&self) -> u32 {
        self.slot
    }

    pub closed spec fn generation_spec(&self) -> u32 {
        self.generation
    }

    #[must_use]
    pub const fn new(slot: u32, generation: u32) -> (request: Self)
        ensures
            request.slot_spec() == slot,
            request.generation_spec() == generation,
    {
        Self { slot, generation }
    }

    #[must_use]
    pub const fn slot(self) -> (slot: u32)
        ensures slot == self.slot_spec(),
    {
        self.slot
    }

    #[must_use]
    pub const fn generation(self) -> (generation: u32)
        ensures generation == self.generation_spec(),
    {
        self.generation
    }
}

} // verus!

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
