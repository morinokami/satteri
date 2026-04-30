---
cargo/satteri-ast: patch
cargo/satteri-plugin-api: patch
npm/satteri: patch
---

Fixed compilation crashing with `invalid type: map, expected a sequence` when an MDAST plugin returned a tree containing a directive node (`containerDirective`, `leafDirective`, `textDirective`). Directive children now round-trip through plugins correctly.
