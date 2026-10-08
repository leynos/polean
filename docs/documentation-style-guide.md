# Documentation style guide

Polean documentation uses British English with Oxford spelling: `-ize` where
both forms exist, `-yse` in words such as “analyse”, and ordinary British forms
such as “behaviour” and “licence”. External identifiers retain their published
spelling.

Prefer direct sentences and active voice. Name the actor, component, theorem,
assumption, or failure mode. Do not use “verified”, “proved”, “safe”, or
“equivalent” without identifying the exact claim and its boundary.

Wrap ordinary Markdown prose near 80 columns. Tables and code may exceed that
limit where wrapping would make them less readable. Use fenced code blocks with
a language where practical. Relative links connect repository documents;
references to external evidence belong in a clearly labelled references
section.

Terms of reference describe the problem, users, scope, assumptions, and
acceptance criteria. Technical designs describe mechanisms and proof
obligations. ADRs record durable decisions, their context, consequences, and
alternatives. Validation records state both what ran and what did not run.

When updating a generated or recorded result, preserve provenance and avoid
wording that promotes tests into proofs. A green workflow is evidence that its
listed checks passed, not a universal theorem about unlisted behaviour.
