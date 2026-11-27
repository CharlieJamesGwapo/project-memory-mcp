# Verify a delivery

Use this route to establish whether a change or artifact meets the requested
outcome. Identify the behavior, revision or workspace, target environment, and
deliverable under assessment. A readiness assessment alone leaves the source
unchanged; when fixes are also authorized, verify again after the final change.

## Choose evidence for the outcome

Read the actual change or artifact through host tools when available. Use saved
decisions to recover acceptance constraints and prior results to identify useful
checks. Preserve the date/revision and author of historical claims; a previous
assistant's passing note is not a result for the present deliverable.

Map each material claim to a bounded check that can establish it. Prefer the
project's relevant existing commands and behavioral tests. A user-flow change
may need its caller or integration path exercised; a packaging claim needs the
produced artifact inspected or run in the authorized environment. Unit tests,
builds, source review, and artifact checks answer different questions. Choose
the ones the requested outcome needs rather than a universal test checklist.

When adding a check is in scope, assert the externally observable requirement
using representative input and a meaningful failure/boundary. Avoid assertions
that simply mirror implementation details or return a predetermined success.
Without execution tools, assess supplied source/results and identify the
specific checks still needed. Without source access, state the inspected
artifact or supplied evidence precisely instead of implying a code review.

## Run and inspect

Run supported checks against the final relevant state. Record the command,
scope, exit status and meaningful result, including failures, skipped checks,
or partial output. A command that started is pending until its result is known.
An unavailable harness or dependency remains an unperformed check; do not
substitute a saved result or describe it as passing.

For a failure, connect the result to the requirement it blocks. Distinguish a
confirmed behavior defect from an environment problem or unexplained failure.
If fixing is authorized, use the debugging route for the cause, then rerun the
affected checks on the changed state. Otherwise report the next useful action.
Repeat or broaden checks when changes, failures, or a specific unresolved risk
justify it. Once appropriate checks pass, proceed to the requested handover.

## State readiness with its evidence

Return the verified behavior or blocking failure first, followed by current
commands/results and material coverage gaps. Distinguish your observations,
user-supplied current receipts, and attributed historical records. A source-only
assessment can support a narrow conclusion while execution remains unverified;
passing a bounded suite supports its scope rather than universal correctness.

Name any remaining check that changes readiness, with the smallest practical
next action. Save results only when the user requests a decision or handoff;
the assessment itself does not publish, deploy, or create a memory record.
