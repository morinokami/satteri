---
npm/satteri: minor
cargo/satteri-pulldown-cmark: minor
cargo/satteri-ast: patch
---

Fix MDX import/export and expression handling to match the behavior of the original JavaScript implementation:

- Fix `mdxjsEsm` nodes not being delivered to HAST plugin visitors
- Fix multiline `export` blocks (e.g. objects, arrays) being truncated
- Fix expression boundaries for edge cases involving comments, template literals, regex, and JSX
- Report errors for unclosed MDX expressions
