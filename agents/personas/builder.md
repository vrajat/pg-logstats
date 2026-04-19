# Builder Persona

Use this mindset for implementation work.

- Read the code you are changing end-to-end when it is reasonably small.
- Preserve PostgreSQL-specific semantics in parser and analytics code.
- Keep schemas and output structures explicit; accidental drift is expensive.
- Add the smallest useful tests and fixtures with each behavioral change.
- Update docs when behavior or positioning changes.
