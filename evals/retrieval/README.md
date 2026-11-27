# Reproduce the retrieval checks

This evaluation exercises a trusted built server through real MCP stdio calls.
It compares literal and ranked-term bundles on the same synthetic project data,
query, byte cap and record cap. It calls no model or network API.

From a source checkout with the terms feature:

```sh
cargo build --locked
python3 scripts/evaluate-retrieval.py /absolute/path/to/target/debug/project-memory-mcp
```

Use the executable from the same checkout; older releases without `query_mode`
are rejected. Only point the command at a binary you trust. The temporary
archive isolates evaluation data, not the supplied executable's OS permissions.
There is no user-database argument or output-file overwrite option. Successful
observations are printed as JSON on stdout; a failed gate exits nonzero with a
message on stderr and no successful report.

## What v1 covers

`v1.json` expands to six projects and 214 records. A `repeat` count creates
named aliases such as `weak-000`, avoiding hundreds of repeated fixture entries.

| Case | Required behavior |
|---|---|
| Distributed fields | Terms recover a record whose task words occur in different fields; the literal phrase does not match. |
| Rank before window | An older record covering both terms outranks 205 newer weak matches before the 200-candidate cap. |
| Ordinary phrase | Both modes return the same relevant complete record. |
| History/source isolation | Superseded decisions, source-only strings and a matching record in another project stay out. |
| Byte-cap omission | Both modes skip an oversized record whole and retain the complete smaller record within 4096 bytes. |

Every named case requires explicit expectations for both modes, including the
exact ordered `matched_terms` for each selected terms-mode record. Missing cases,
unknown/foreign record aliases, unbounded expansion and incomplete expectations
are rejected. Independent required project/record schemas validate canonical
UUIDs, valid RFC3339 timestamps, attribution, sources and supersession fields
before accepting a seed response as an original. This catches a serializer
omitting the same required field in both save and retrieval responses. Seeding
also checks supplied fields, then retrieved records must equal their complete
originals, including authorship, sources, timestamps and explicit decision
history. Matched-term lists must equal the hand-authored fixture expectations,
not merely be an ordered subset of the query. Selection and omission counters must match the
fixture, and returned records must belong to the selected project.
These checks validate schemas and preservation of synthetic fixture values;
they do not fetch/authenticate sources or prove a recorded claim true.

## Read the measurements

The report includes the fixture version and SHA-256, server identity, named
selected records, relevant records returned/defined, candidate-window status,
omission counts and canonical MCP result bytes. Random UUIDs are mapped to fixed
aliases so selections can be compared across runs. Byte counts can vary slightly
with server timestamps; they are observations, not frozen expected totals.

Canonical bytes are measured independently from compact UTF-8 JSON of the actual
tool result, including its structured and JSON-text compatibility copies. For a
legacy negotiated protocol that omits `resultType`, the checker restores
`"resultType":"complete"` before counting, matching the server's conservative
budget contract. The outer JSON-RPC envelope is excluded. The checker rejects
disagreement between compatibility text and structured content, incorrect byte
accounting, cap overshoot, altered originals, cross-project records or incorrect
fixture selections.

These are deterministic synthetic regression cases. Literal matching and ranked
terms serve different purposes; terms can return more relevant records and use
more bytes. This is not an independent quality benchmark, a model-token usage
measurement, a billing comparison or a universal savings/relevance claim. The
client's full conversation and tool overhead are outside the measurement.

Checker rejection tests are separate from the real-MCP evaluation:

```sh
python3 -m unittest discover -s tests -p 'retrieval_evaluation_test.py'
```

Those tests use controlled protocol fixtures to test validation failures. They
are never reported as successful real-MCP evaluation runs. Keep the v1 fixture
version and exact hash with any shared result so changes remain reviewable.
