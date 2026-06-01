---
cargo/satteri-arena: patch
cargo/satteri-ast: patch
npm/satteri: patch
---

Fixes Markdown plugins that return raw Markdown or HTML (`{ raw }` / `{ rawHtml }`) sometimes inserting unnecessary nested `root` nodes into the MDAST tree.
