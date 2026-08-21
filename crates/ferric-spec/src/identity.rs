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

    /// Ghost constructor for a complete 32-byte identity view.
    pub closed spec fn from_bytes_spec(bytes: Seq<u8>) -> Self
        recommends bytes.len() == 32,
    {
        Self([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
            bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31],
        ])
    }

    /// Exposes the byte view of the ghost constructor.
    pub proof fn from_bytes_spec_view(bytes: Seq<u8>)
        requires bytes.len() == 32,
        ensures Self::from_bytes_spec(bytes).bytes_spec() == bytes,
    {
        assert(Self::from_bytes_spec(bytes).bytes_spec() =~= bytes) by {
            assert forall|index: int| 0 <= index < 32 implies
                Self::from_bytes_spec(bytes).bytes_spec()[index] == bytes[index] by {
                if index == 0 {} else if index == 1 {} else if index == 2 {}
                else if index == 3 {} else if index == 4 {} else if index == 5 {}
                else if index == 6 {} else if index == 7 {} else if index == 8 {}
                else if index == 9 {} else if index == 10 {} else if index == 11 {}
                else if index == 12 {} else if index == 13 {} else if index == 14 {}
                else if index == 15 {} else if index == 16 {} else if index == 17 {}
                else if index == 18 {} else if index == 19 {} else if index == 20 {}
                else if index == 21 {} else if index == 22 {} else if index == 23 {}
                else if index == 24 {} else if index == 25 {} else if index == 26 {}
                else if index == 27 {} else if index == 28 {} else if index == 29 {}
                else if index == 30 {} else {}
            }
        }
    }

    /// Establishes the exact width of the canonical identity view.
    pub proof fn bytes_spec_len(&self)
        ensures self.bytes_spec().len() == 32,
    {
    }

    /// Establishes identity equality from equality of the complete byte view.
    pub proof fn extensional(left: &Self, right: &Self)
        requires left.bytes_spec() == right.bytes_spec(),
        ensures *left == *right,
    {
        assert(left.0 =~= right.0) by {
            assert forall|index: int| 0 <= index < 32 implies left.0[index] == right.0[index] by {}
        }
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

    /// Compares two identities without exposing their bytes.
    #[must_use]
    pub fn equals(&self, other: &Self) -> (equal: bool)
        ensures equal == (self.bytes_spec() == other.bytes_spec()),
    {
        let mut index = 0;
        while index < self.0.len()
            invariant
                self.bytes_spec().len() == other.bytes_spec().len(),
                0 <= index <= self.bytes_spec().len(),
                forall|prior: int|
                    0 <= prior < index
                        ==> self.bytes_spec()[prior] == other.bytes_spec()[prior],
            decreases self.bytes_spec().len() - index,
        {
            if self.0[index] != other.0[index] {
                return false;
            }
            index += 1;
        }
        assert(self.bytes_spec() =~= other.bytes_spec()) by {
            assert forall|position: int|
                0 <= position < self.bytes_spec().len()
                    implies self.bytes_spec()[position] == other.bytes_spec()[position]
            by {}
        }
        true
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

    #[test]
    fn identity_equality_checks_every_byte() {
        assert!(Identity::new([7; 32]).equals(&Identity::new([7; 32])));

        let mut different = [7; 32];
        different[31] = 8;
        assert!(!Identity::new([7; 32]).equals(&Identity::new(different)));
    }
}
