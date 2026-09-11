//! Engineering geometry only; not an authority or memory-availability guarantee.

pub const MAX_ROWS: u32 = 32;
pub const PAGE_TOKENS: u32 = 16;
pub const MAX_PHYSICAL_PAGES: u32 = 16384;
pub const MAX_LOGICAL_PAGES: u32 = 512;
pub const MAX_CONTEXT_TOKENS: u32 = 8192;
pub const MAX_PHYSICAL_SLOTS: u32 = MAX_PHYSICAL_PAGES * PAGE_TOKENS;
pub const KV_COLUMNS: u32 = 1024;
pub const MAX_CACHE_ELEMENTS: u32 = MAX_PHYSICAL_SLOTS * KV_COLUMNS;
pub const WORKGROUP: [u32; 3] = [64, 1, 1];
pub const APPEND_EXPLICIT_BYTES: u32 = 112;
pub const ATTENTION_EXPLICIT_BYTES: u32 = 116;

pub const fn supported(rows: u32, world: u32, stride: u32, pages: u32) -> bool {
    rows > 0
        && rows <= MAX_ROWS
        && world == 1
        && stride > 0
        && stride <= MAX_LOGICAL_PAGES
        && pages > 0
        && pages <= MAX_PHYSICAL_PAGES
}
