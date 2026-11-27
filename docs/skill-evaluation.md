# Evaluate the workflow pack

The pack now has focused debugging and delivery-verification references, loaded
only when selected through `project-memory`. The four installed skill folders
remain the same. These are discoverable workflow guides; they do not create an
agent runtime, execution tools or permission to edit, deploy or publish.

## Versioned task fixtures

The source repository's [`evals/project-memory`](https://github.com/CharlieJamesGwapo/project-memory-mcp/tree/main/evals/project-memory) contains synthetic
versioned inputs and separate evaluator criteria. Copy only one scenario's
`input/` into an isolated workspace. Give a fresh worker the task, chosen skill
pack and actual host capabilities. Keep the sibling `criteria.json`, previous
results and suspected correction out of that worker's context.

Read the response, actual commands/exits and changed files against the criteria.
For a baseline, snapshot the old pack before editing. Use the same task and
capabilities for the forward case. Save receipts outside the source tree and
record the pack revision, fixture version, prompt, commands, results, response
and criterion-by-criterion observations. The failing Python tests in fixture
inputs are intentional scenarios, not failures in the product's normal suite.

## Observed checks for this upgrade

Two scenarios ran against the previous v0.2.1 pack and the revised pack, each in
a fresh worker with copied synthetic files and real local execution. Live
Project Memory was unavailable. All responses and receipts were read against
the withheld criteria, including source changes or preservation.

| Scenario | Previous pack | Revised pack | Observation |
|---|---|---|---|
| Debug stale-handoff pagination | 6/6 criteria met | 6/6 criteria met | Current tests reproduced the inclusive cursor defect; a narrow correction made all four fixture checks pass. Historical client-side claims were not treated as current proof. |
| Assess failed verification | 6/6 criteria met | 6/6 criteria met | Both reported whitespace handling as not ready, retained the implementation, executed current checks and named missing integration/packaging evidence. |

The baseline already succeeded. These runs found no model misconduct and do
not establish behavioral improvement, model-wide reliability, token savings or
reduced billing. The new routes make their procedures directly discoverable.
Structural frontmatter/reference validation is a different check from behavior.

The supplied unavailable-tools scenario remains **unrun** in a genuinely
tool-disabled host. A tool-enabled worker asked to pretend tools are absent
would not establish that capability boundary. Use an appropriate host before
reporting that evaluation as complete.

For mechanical pack checks, run `python3 scripts/check-skills.py`. For broader
methodology, see the [Agent Skills evaluation guide](https://agentskills.io/skill-creation/evaluating-skills).
