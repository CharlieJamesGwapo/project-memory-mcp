# Plan meaningful tests

Use for selecting or implementing checks for a specified change. Establish the
behavioral contract, affected callers and existing test coverage before choosing
a test layer. A planning request produces a plan; it does not authorize edits.

Map each material claim to a trigger, expected observable result and cheapest
reliable check. Cover the changed behavior and its relevant refusal boundary:
for a scoped memory read, test the allowed project and a denied direct call;
for pagination, test interleaved rows and a second page. Add concurrency or
integration checks where the behavior actually crosses that boundary.

Prefer an existing reproduction or focused regression that fails on the defect
and passes after correction. Avoid assertions that duplicate implementation,
mock away the behavior under test, or merely count checks. State which fixtures
are synthetic and keep user archives out of tests. Separate expected rejection
from infrastructure failures.

Return a short claim-to-check mapping, actual results when executed, and the
important uncovered behavior. A model cannot run tests without host execution
tools. Saved test receipts remain historical until relevant checks are rerun.
