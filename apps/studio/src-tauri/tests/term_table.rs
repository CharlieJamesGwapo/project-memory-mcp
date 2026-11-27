use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TermTable {
    unicode_version: [u8; 3],
    ranges: Vec<[u32; 2]>,
}

#[test]
fn frontend_term_ranges_match_every_rust_unicode_scalar() {
    let table: TermTable =
        serde_json::from_str(include_str!("../../src/term-ranges.json")).unwrap();
    let version = char::UNICODE_VERSION;
    assert_eq!(table.unicode_version, [version.0, version.1, version.2]);
    assert!(!table.ranges.is_empty());
    for (index, [start, end]) in table.ranges.iter().copied().enumerate() {
        assert!(start <= end && end <= 0x10ffff);
        assert!(
            end < 0xd800 || start > 0xdfff,
            "surrogate range is not a scalar"
        );
        if let Some(previous) = index.checked_sub(1) {
            assert!(
                table.ranges[previous][1] + 1 < start,
                "ranges must be ordered and coalesced"
            );
        }
    }
    let mut index = 0;
    for point in 0..=0x10ffff {
        let Some(character) = char::from_u32(point) else {
            continue;
        };
        while index < table.ranges.len() && table.ranges[index][1] < point {
            index += 1;
        }
        let classified = table
            .ranges
            .get(index)
            .is_some_and(|[start, end]| *start <= point && point <= *end);
        assert_eq!(classified, character.is_alphanumeric(), "U+{point:04X}");
    }
}
