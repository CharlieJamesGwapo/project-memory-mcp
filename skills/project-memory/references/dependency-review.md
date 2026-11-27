# Review dependencies

Establish the exact manifest and lockfile change, target platforms, enabled
features and the active dependency path. Separate runtime, build and optional
lockfile entries; a locked crate's presence alone does not establish runtime
reachability. A transitive warning still deserves its actual scope, not dismissal.

Use available current advisory evidence and official changelogs for the exact
versions. Record the scanner/version, advisory snapshot or retrieval date, target
and result when those checks actually run. Distinguish vulnerability, unsoundness,
unmaintained status and compatibility concerns. A clean scan does not inspect
all source or establish malware absence.

For each actionable issue report the introducing dependency path, affected
version/platform, relevant behavior and a supported remediation. Assess whether
an upgrade fits the existing dependency family; do not invent a patched version.
Review lock origins, new install/build scripts and license changes when relevant.
Do not run an untrusted installer merely to review it.

A review alone leaves locks and dependencies unchanged. For authorized upgrades,
make the smallest compatible change and verify affected builds and behavior.
Report actual checks and remaining reachability or platform uncertainty. Save a
decision or review only within the user's requested memory-write scope.
