//! Host-only arithmetic model. Device kernels always use fe2o3's Bf16.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bf16(u16);

impl Bf16 {
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn to_bits(self) -> u16 {
        self.0
    }

    pub const fn to_f32(self) -> f32 {
        f32::from_bits((self.0 as u32) << 16)
    }

    pub const fn from_f32(value: f32) -> Self {
        let bits = value.to_bits();
        if bits & 0x7fff_ffff > 0x7f80_0000 {
            return Self(((bits >> 16) as u16) | 0x0040);
        }
        let even_bias = 0x7fff + ((bits >> 16) & 1);
        Self((bits.wrapping_add(even_bias) >> 16) as u16)
    }

    pub const fn is_finite(self) -> bool {
        self.0 & 0x7f80 != 0x7f80
    }
}
