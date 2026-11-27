# Review a code change

Use available source/diff tools to establish what changed and what behavior the
user wants reviewed. Resolve the base commit, working-tree scope, or supplied
patch before assessing it. If the actual scope is empty, report that observation.
If the base is ambiguous and changes the review, clarify it; otherwise use the
obvious supplied scope and name it. A review request does not itself request edits.

For commit-based review, resolve exact base/head object IDs and the repository
label from the current host. Do not invent commit hashes for uncommitted work.
When `get_review_history` is available, discover its actual schema and retrieve
only the relevant project's reviews. Compare their recorded head with the
current head. A stale review is historical evidence, not proof that a finding
is fixed or that the current change passes. A matching hash still does not
certify the supplied checks were genuine or sufficient.

When advertised, `review_change` is an optional workflow prompt requiring the
resolved `project_id` and optional `task` (up to 2000 characters). Discover its
arguments; generating it does not retrieve a review, inspect a diff, run checks
or authorize a save. A permitted `get_memory_health` report may point to review
follow-up or a supplied-head mismatch, but does not replace complete review
history or current source evidence. The default health resource has no supplied
head comparison. If these views are absent, retain the permitted review/read
tool workflow; a hidden capability is not proof of an older server.

Recover Project Memory only when it supplies a decision relevant to the change.
Saved source references and prior check results are pointers, not current proof.
Read surrounding implementation and callers needed to understand the changed
behavior. Inspect relevant tests and run bounded checks when the host and task
permit. Do not install dependencies or run an untrusted installer just to review
its source. Without execution tools, a concrete source-derived counterexample
can still support a finding; identify checks that were not run.

For each actionable finding provide:

- Severity consistent with the actual impact, and a short description.
- A verified path and tight changed-line location (or explicitly identified
  surrounding code if the defect is outside the diff).
- The concrete trigger, resulting behavior, and why it violates the requirement.
- Actual source, call-site, or reproduction evidence and a useful correction.

One root cause can have several examples; avoid duplicate findings. Distinguish
observed faults from unanswered questions and optional suggestions. Review the
claim against the actual code before flagging it. If no actionable fault is
found, say so with scope and remaining verification limits; do not manufacture a
finding or imply exhaustive correctness.

Example: a change from `active and (admin or member)` to
`active or (admin or member)` admits an active outsider and an inactive member.
Check the actual guard and its requirement, then report one authorization finding
at the changed line; do not claim a production exploit from a synthetic example.

Finish with findings first, then current check results and relevant limitations.
Confirm independent review only if another agent actually reviewed this change.

If the user requests saving the review, discover `save_review` and its schema,
check available policy/read-only restrictions, and save within that exact
project. Include the actual commits, attributed summary, checks and each
finding's stable ID, location, evidence, correction and status. Use the server's
documented enum values and limits. Status `fixed` needs current correction
evidence; acceptance is a decision, not a verified fix. Link a revision to the
exact previous review and preserve history. Confirm saving only after success.

On a confirmed older server without review capability, use an explicitly labeled
ordinary note fallback through available permitted save tools for an already
requested save; preserve the evidence and disclose the missing structured
history. Hidden or denied review tools are not evidence of an older server;
if the capability/policy distinction is unknown, report the limitation instead
of bypassing it through another write operation. Without full
commits, report that structured saving lacks required evidence rather than
fabricating IDs. A denied write does not authorize changing policy, bypassing
the server or saving through another connection. Keep sensitive values out of
findings and saved summaries; preserve their safe locations/categories instead.
