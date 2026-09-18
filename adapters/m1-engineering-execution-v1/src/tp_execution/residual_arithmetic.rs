//! Explicit host arithmetic ablations, not a device or numerical qualification.

use super::{
    EngineeringTpExecutionV1, EngineeringTpRankTransportV1, EngineeringTpReductionModeV3,
    HostStagedPartialV1, TpResult, reduce_residual_bf16_v1,
};

/// Rounding placement after the complete, ascending-rank FP32 projection sum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EngineeringTpResidualArithmeticV1 {
    /// Add the BF16 residual to the FP32 sum, then round once to BF16.
    #[default]
    Fp32ResidualV1,
    /// Materialize the projection as BF16 before the separate residual addition.
    ProjectionBf16V1,
}

impl EngineeringTpResidualArithmeticV1 {
    /// Parses an exact arithmetic identity; unknown profiles never fall back.
    /// # Errors
    /// Rejects any identity other than the two named host profiles.
    pub fn parse(value: &str) -> TpResult<Self> {
        match value {
            "fp32-rank-sum-plus-residual-then-bf16-v1" => Ok(Self::Fp32ResidualV1),
            "fp32-rank-sum-then-bf16-plus-residual-then-bf16-v1" => Ok(Self::ProjectionBf16V1),
            _ => Err("unsupported host residual arithmetic".into()),
        }
    }

    /// Stable arithmetic identity, independent of host staging allocation policy.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fp32ResidualV1 => "fp32-rank-sum-plus-residual-then-bf16-v1",
            Self::ProjectionBf16V1 => "fp32-rank-sum-then-bf16-plus-residual-then-bf16-v1",
        }
    }

    pub(super) fn projection(self, sum: f32) -> TpResult<f32> {
        if self == Self::Fp32ResidualV1 {
            return Ok(sum);
        }
        let bits = round_bf16(sum, "BF16 projection sum overflow")?;
        Ok(f32::from_bits(u32::from(bits) << 16))
    }
}

fn round_bf16(value: f32, error: &str) -> TpResult<u16> {
    if !value.is_finite() {
        return Err(error.into());
    }
    let bits = value.to_bits();
    let rounding = 0x7fff + ((bits >> 16) & 1);
    let rounded = (bits.wrapping_add(rounding) >> 16) as u16;
    if !f32::from_bits(u32::from(rounded) << 16).is_finite() {
        return Err(error.into());
    }
    Ok(rounded)
}

pub(super) fn reduce(
    mode: EngineeringTpResidualArithmeticV1,
    world: u32,
    partials: &[HostStagedPartialV1<'_>],
    residual: &[u16],
) -> TpResult<Vec<u16>> {
    if mode == EngineeringTpResidualArithmeticV1::Fp32ResidualV1 {
        return reduce_residual_bf16_v1(world, partials, residual);
    }
    // A zero residual reuses the original roster checks and ordered FP32 sum.
    // No rank-local partial is rounded before that complete reduction.
    let projection = reduce_residual_bf16_v1(world, partials, &vec![0; residual.len()])?;
    projection
        .into_iter()
        .zip(residual)
        .map(|(projection, residual)| {
            let projection = f32::from_bits(u32::from(projection) << 16);
            let residual = f32::from_bits(u32::from(*residual) << 16);
            if !residual.is_finite() {
                return Err("nonfinite residual sum".into());
            }
            round_bf16(projection + residual, "BF16 residual sum overflow")
        })
        .collect()
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    /// Selects host rounding once, before any dispatch. Device collectives and
    /// legacy numerical captures do not admit this arithmetic ablation. The separate
    /// arithmetic final-stage capture binds the exact explicit mode before dispatch.
    ///
    /// # Errors
    /// Rejects a closed/started stream, repeated selection or a device reduction.
    pub fn configure_host_residual_arithmetic(
        &mut self,
        mode: EngineeringTpResidualArithmeticV1,
    ) -> TpResult<()> {
        if self.closed
            || self.ranks.iter().any(|rank| rank.dispatches != 0)
            || self.residual_arithmetic.is_some()
            || !matches!(
                self.reduction.mode(),
                EngineeringTpReductionModeV3::HostStagedV1
                    | EngineeringTpReductionModeV3::HostStagedReuseV3
            )
        {
            return Err(
                "host residual arithmetic requires fresh, unconfigured host reduction".into(),
            );
        }
        self.residual_arithmetic = Some(mode);
        Ok(())
    }

    /// Active arithmetic identity for setup records.
    #[must_use]
    pub const fn residual_arithmetic(&self) -> EngineeringTpResidualArithmeticV1 {
        match self.residual_arithmetic {
            Some(mode) => mode,
            None => EngineeringTpResidualArithmeticV1::Fp32ResidualV1,
        }
    }
}

#[cfg(test)]
mod tests;
