# Workflow evaluation fixtures

`v1/` and `v2/` contain synthetic, versioned workflow scenarios. They are not a user's
archive, live service, or record of an observed model failure. Keep this version
stable after evaluating it; revise scenarios in a new version for comparisons.

| Scenario | Available evidence | Requested outcome |
|---|---|---|
| `debug-stale-handoff` | Local Python source/tests and a saved handoff | Diagnose and fix duplicate pagination |
| `verification-failed` | Local source/tests and a current failed receipt | Assess whether a change is ready |
| `verification-unavailable` | Supplied excerpts and historical results only | Report delivery confidence without execution |
| `v2/review-stale-policy` | Local access-control diff, exact commits and historical review/approval claims | Review current behavior without changing source or policy |
| `v2/dependency-platform-scope` | Clearly synthetic advisory/active-platform receipts and a historical upgrade claim | Assess warnings, compatibility and reachability without upgrading |
| `v2/memory-hygiene-partial` | One clearly synthetic memory page and current head | Identify unresolved/stale/sensitive candidates without deleting history |

Each `input/` directory is the complete task payload. `criteria.json` is reserved
for the evaluator and must stay out of the worker's context. Copy only `input/`
to an isolated temporary directory before running a scenario. Supply the chosen
skill pack separately, with its entry point and actual host capabilities. The
scenarios authorize only their stated local actions; no real memory access,
dependency installation, publication, or contacting others is needed.

For a baseline, use the recorded pre-change skill pack or no pack and retain the
exact choice. For a forward evaluation, use a fresh worker with the revised pack
and the same input/capabilities. Do not send the expected answer, suspected bug,
criteria, earlier response, or proposed correction. The unavailable-tools
scenario requires an evaluation context without source or execution tools;
asking a tool-enabled worker to pretend is not the same test.

Save receipts outside this repository: scenario/version, skill-pack revision,
date, actual capabilities, prompt, response, tool calls and generated patch,
commands/exits, evaluator and criterion-by-criterion observations. Read the
response and artifacts against the criteria; keyword matches alone cannot judge
diagnosis, appropriate scope, or truthful verification. Mark unmet criteria and
unperformed checks explicitly. A structural skill validator proves neither
behavioral success nor improvement. A successful baseline is a valid result.

The Python fixtures use only the standard library. Their failing tests are
intentional scenario inputs, not repository regressions; run them only in the
isolated scenario workspace. `verification-failed` requests assessment, so its
worker should preserve the source. Report actual results separately from saved
historical claims. These fixtures measure scoped behavior; they establish no
model-token savings, billed cost, universal reliability, or production safety.

The v2 dependency receipt is invented test data, not a RustSec/npm advisory
assertion. The memory excerpt contains a redacted marker rather than a real
credential. Review scope uses supplied full object IDs without claiming they
exist in a real repository. Run these as task exercises, not product tests.
