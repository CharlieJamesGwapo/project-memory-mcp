# Reproduce retrieval checks

The source repository includes a versioned synthetic evaluation that calls a
trusted local server over real MCP stdio. It compares literal and ranked terms
on the same data, query and caps, without a model or network API.

From a matching v0.3 source checkout:

```sh
cargo build --locked
python3 scripts/evaluate-retrieval.py /absolute/path/to/target/debug/project-memory-mcp
```

The helper creates its own temporary archive; it has no user-database option.
It prints a JSON report only after all gates pass. A failed gate exits nonzero.
Use a binary you trust: temporary data does not sandbox executable permissions.
The fixtures and evaluator are development resources in the source checkout;
they are not required to run the prebuilt server or installed skill pack.

The [fixture guide](https://github.com/CharlieJamesGwapo/project-memory-mcp/tree/main/evals/retrieval)
describes all five cases: distributed task words, an older strong match ahead of
205 newer weak matches, an ordinary phrase, history/source/project isolation,
and whole-record byte-cap omissions. v1 seeds six projects and 214 records.

The checker independently measures canonical MCP result bytes, checks the text
and structured representations agree, verifies original fields and provenance,
and requires explicit selected-record and omission expectations for both modes.
Missing expectations, altered records, leakage, incorrect byte accounting and
cap overshoot fail the evaluation. Random UUIDs map to stable aliases for comparison;
observed byte counts can vary slightly with timestamps.

The release workflow runs this helper against the actual unpacked MCPB and
includes `retrieval-evaluation.json` with release checksums and attestations.
This is a synthetic protocol/selection regression receipt, not an independent
quality benchmark. Terms can return more relevant fixture records and more
bytes. Neither this comparison nor bytes/4 measures actual model usage, billed
cost or universal savings. See [the API contract](api-reference.md) and
[skill evaluation](skill-evaluation.md) for their separate scopes.
