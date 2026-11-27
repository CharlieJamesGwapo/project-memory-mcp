# Debug a reported failure

Use this route for a bug, failed check, or unexpected behavior. Start with the
expected behavior, observed result, trigger, and affected revision/environment
from the user's report. Inspect available evidence to fill gaps; ask only for
missing information that prevents a useful reproduction or changes the fix.

## Establish the current failure

Use relevant saved decisions or a handoff to recover intent, a prior hypothesis,
or source pointers. Retrieve narrowly when live memory is available. A saved
"fixed" or "tests passed" statement describes its recorded state; compare it
with the current source and results before choosing a cause.

Follow the actual input through the relevant caller and implementation using
host source tools. Read nearby tests and configuration needed to reproduce the
trigger. Choose the smallest local reproduction that distinguishes the expected
behavior from the observed one. Capture its input, result, command/exit when
executed, and revision or workspace scope. Preserve useful failure output.

If execution is unavailable, trace a concrete input through supplied source and
label the result as source-derived. Identify what execution would still need to
confirm. If source is also unavailable, distinguish the reported symptom from a
working hypothesis and request the narrow missing artifact needed to proceed.

## Connect evidence to a correction

Explain the causal path: triggering input → responsible operation → incorrect
output or state. Compare competing explanations against current evidence rather
than applying the last handoff's suggestion by default. An unexplained failing
assertion or a missing dependency identifies a next investigation, not yet the
product's root cause. Name any unresolved cause explicitly.

When a fix is authorized, make the smallest correction supported by that cause.
Use an existing reproducing behavioral test when it covers the defect. Add a
focused regression case when the important trigger is uncovered; make it fail
for the observed behavior before changing the implementation. Include a nearby
boundary when it distinguishes a plausible incomplete fix. Assertions should
check observable results, not copy the new expression or require its spelling.

Run the reproduction after the change and the relevant existing checks through
available host tools. Inspect results and exits before describing the outcome.
A failed check calls for an evidence-based next step; an unavailable dependency
or environment limits verification. Stop expanding tests once appropriate
checks pass unless another change, failure, or unresolved concern warrants it.

## Return the useful result

Report the cause and supporting source/reproduction, the correction made or
recommended, and actual current checks with remaining limits. Keep saved claims
attributed separately. For diagnosis-only work, give the supported correction
without editing. The workflow itself adds no memory write; use an explicitly
requested decision or handoff save only when that is part of the user's task.
