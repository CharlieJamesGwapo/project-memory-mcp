# Refactor while preserving behavior

Resolve the requested scope and the behaviors that must remain stable: public
inputs/results, errors, persistent representation and important side effects.
Use saved decisions as context and current callers/tests as evidence. Distinguish
a refactor from a requested behavior change before modifying the contract.

Identify the smallest useful boundary to improve. Add a characterization or
regression check only when existing coverage cannot detect a material change.
Preserve authorization, transaction and output-budget enforcement at the layer
that owns them; moving a prompt rule is not equivalent to a server-side guard.

Make small coherent changes and run checks that exercise the preserved behavior.
For an archive or tool contract, compare actual externally visible results and
refusal paths rather than relying solely on private-method tests. Keep schema
changes and unrelated dependency upgrades outside a behavior-preserving refactor
unless the user explicitly requests that wider work.

Report what became simpler, which contracts were preserved and what was actually
checked. If a check fails, investigate before describing the refactor as ready.
Commit, push, publication and saved review records follow the user's existing
authorization; source-edit permission alone does not establish all of these.
