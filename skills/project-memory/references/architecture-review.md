# Review architecture

Start with the user's concrete goals and constraints, current entry points,
state ownership and trust boundaries. Inspect actual modules, dependencies and
data flow before describing the system. Saved architectural choices explain
intent; they do not prove current implementation matches it.

Evaluate only tradeoffs relevant to the request: failure isolation, concurrency,
data lifetime, compatibility, operational cost and access control. Trace at least
one representative operation across the relevant boundary. For Project Memory,
distinguish host model/agent execution, MCP memory operations, Store enforcement
and local file access; per-connection project policy is not multi-user isolation.

Present concrete observations first, then a small set of viable alternatives
with their consequences. Prefer the smallest change that meets the stated
goals. Do not prescribe a graph database, cloud sync, microservices or a schema
migration merely because a larger system could use them.

Return the recommended boundary/change, supporting source locations, important
tradeoffs and a check that would validate the proposal. Label untested capacity
or latency estimates. A design assessment does not itself request refactoring;
record an architectural decision only when the user actually chooses it or
requests an attributed proposal to be saved.
