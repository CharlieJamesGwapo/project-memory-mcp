//! Generate the numeric character ranges used by the Studio task tokenizer.
//! Run with the same pinned toolchain as the server; no JavaScript engine or
//! external Unicode data is used:
//!
//! rustc +1.100.0 --edition=2024 scripts/generate-term-table.rs -o /tmp/project-memory-term-table
//! /tmp/project-memory-term-table > apps/studio/src/term-ranges.json
//!
//! The native term_table integration test checks this output against every
//! Unicode scalar and the toolchain's recorded Unicode version.

use std::io::{self, BufWriter, Write};

fn main() -> io::Result<()> {
    let mut ranges = Vec::new();
    let mut start = None;
    let mut end = 0;
    for point in 0..=0x10ffff {
        if char::from_u32(point).is_some_and(char::is_alphanumeric) {
            start.get_or_insert(point);
            end = point;
        } else if let Some(first) = start.take() {
            ranges.push([first, end]);
        }
    }
    if let Some(first) = start {
        ranges.push([first, end]);
    }
    let version = char::UNICODE_VERSION;
    let mut output = BufWriter::new(io::stdout().lock());
    writeln!(output, "{{")?;
    writeln!(
        output,
        "  \"unicode_version\": [{}, {}, {}],",
        version.0, version.1, version.2
    )?;
    writeln!(output, "  \"ranges\": [")?;
    for (index, [first, last]) in ranges.iter().enumerate() {
        let comma = if index + 1 == ranges.len() { "" } else { "," };
        writeln!(output, "    [{first}, {last}]{comma}")?;
    }
    writeln!(output, "  ]\n}}")?;
    output.flush()
}
