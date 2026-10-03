//! Setup-only transfer sizing; kernel dispatch and decode metadata are unchanged.

use super::{EngineeringTpRankTransportV1, TpResult};
use ferric_engine::tensor_parallel::Qwen3TensorParallelTensorV1;

const MAX_TRANSFER_BYTES: usize = 4 << 20;

fn chunk_bytes(transport: &impl EngineeringTpRankTransportV1) -> TpResult<usize> {
    let bytes = transport.setup_upload_chunk_bytes();
    if !(2..=MAX_TRANSFER_BYTES).contains(&bytes) || !bytes.is_multiple_of(2) {
        return Err("invalid setup upload chunk capability".into());
    }
    Ok(bytes)
}

#[cfg(any(feature = "tp-batch-engineering", test))]
pub(super) fn write_contiguous(
    transport: &mut impl EngineeringTpRankTransportV1,
    buffer: u64,
    source: &[u8],
) -> TpResult<()> {
    let chunk = chunk_bytes(transport)?;
    for (index, bytes) in source.chunks(chunk).enumerate() {
        transport.write(buffer, index * chunk, bytes)?;
    }
    Ok(())
}

pub(super) fn write_shard(
    transport: &mut impl EngineeringTpRankTransportV1,
    buffer: u64,
    source: &[u8],
    shard: &Qwen3TensorParallelTensorV1,
) -> TpResult<()> {
    let bytes = chunk_bytes(transport)?;
    let row_bytes = usize::try_from(shard.columns().count)
        .map_err(|_| "shard column count overflow")?
        .checked_mul(2)
        .ok_or("shard row bytes overflow")?;
    let rows = usize::try_from(shard.rows().count).map_err(|_| "shard row count overflow")?;
    if row_bytes == 0 || row_bytes > MAX_TRANSFER_BYTES {
        return Err("shard row exceeds existing transfer limit".into());
    }
    // Keep complete BF16 rows, including the legacy one-row minimum.
    let chunk_rows = (bytes / row_bytes).max(1);
    let mut chunk = vec![0; chunk_rows.min(rows) * row_bytes];
    for row_start in (0..rows).step_by(chunk_rows) {
        let count = chunk_rows.min(rows - row_start);
        for local in 0..count {
            shard
                .copy_bf16_row_into(
                    source,
                    u32::try_from(row_start + local).map_err(|_| "shard row overflow")?,
                    &mut chunk[local * row_bytes..(local + 1) * row_bytes],
                )
                .map_err(|error| format!("BF16 shard copy: {error:?}"))?;
        }
        transport.write(buffer, row_start * row_bytes, &chunk[..count * row_bytes])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
