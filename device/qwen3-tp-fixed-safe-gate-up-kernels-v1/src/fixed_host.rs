//! Pure geometry model for scoped host fixtures, not device execution.

use crate::host::{KernelGeometry, PackingError};

pub fn validate(geometry: KernelGeometry) -> Result<(), PackingError> {
    if geometry.n != 12_288 || !matches!(geometry.projection, 4 | 5) {
        return Err(PackingError::Shape);
    }
    geometry.validate()
}

pub fn read_indices(column: usize, group: usize, lane: usize) -> Result<[usize; 2], PackingError> {
    if column >= 12_288 || group >= 32 || lane >= 64 {
        return Err(PackingError::Coordinate);
    }
    let inner = group * 64 + lane;
    Ok([inner, column * 2048 + inner])
}
