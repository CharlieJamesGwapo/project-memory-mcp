# Review a code change

Use available source/diff tools to establish what changed and what behavior the
user wants reviewed. Resolve the base commit, working-tree scope, or supplied
patch before assessing it. If the actual scope is empty, report that observation.
If the base is ambiguous and changes the review, clarify it; otherwise use the
obvious supplied scope and name it. A review request does not itself request edits.

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
Save findings only when the user requests a note/decision/handoff; preserve
original authorship and current evidence if a requested handoff includes them.
