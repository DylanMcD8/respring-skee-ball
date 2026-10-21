// Checks coverage by a union of mappings without treating gaps as mapped bytes.
// u64 endpoints allow a mapping to end exactly at the 32-bit address-space limit.
pub(super) fn covered(start: u32, len: u32, ranges: impl Iterator<Item = (u32, u32)>) -> bool {
    let end = u64::from(start) + u64::from(len);
    if len == 0 || end > (1u64 << 32) { return false; }
    let mut ranges: Vec<_> = ranges.map(|(base, size)|
        (u64::from(base), u64::from(base) + u64::from(size))).collect();
    ranges.sort_unstable();
    let mut cursor = u64::from(start);
    for (base, limit) in ranges {
        if limit > (1u64 << 32) || limit <= cursor { continue; }
        if base > cursor { return false; }
        cursor = limit;
        if cursor >= end { return true; }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn inside_one() { assert!(covered(0x1800, 0x1000, [(0x1000, 0x2000)].into_iter())); }
    #[test] fn across_adjacent_unsorted() { assert!(covered(0x1800, 0x2800, [(0x3000,0x1000),(0x1000,0x1000),(0x2000,0x1000)].into_iter())); }
    #[test] fn overlaps() { assert!(covered(0x1000,0x3000, [(0x1000,0x2000),(0x2000,0x2000)].into_iter())); }
    #[test] fn gap() { assert!(!covered(0x1000,0x3000, [(0x1000,0x1000),(0x3000,0x1000)].into_iter())); }
    #[test] fn before_mapping() { assert!(!covered(0x800,0x1000, [(0x1000,0x1000)].into_iter())); }
    #[test] fn after_mapping() { assert!(!covered(0x1800,0x1000, [(0x1000,0x1000)].into_iter())); }
    #[test] fn address_space_end() { assert!(covered(0xfffff000,0x1000, [(0xfffff000,0x1000)].into_iter())); }
    #[test] fn overflow_and_empty() { assert!(!covered(0xfffff000,0x2000, [(0xfffff000,0x2000)].into_iter())); assert!(!covered(1,0, [(0,10)].into_iter())); }
}
