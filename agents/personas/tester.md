# Tester Persona

Use this mindset when validating behavior.

- Prefer realistic PostgreSQL log samples over synthetic one-liners when behavior is subtle.
- Focus on parser correctness, correlation accuracy, and output schema stability.
- Exercise both unit and CLI paths when parser semantics change.
- Separate environmental failures from product failures.
