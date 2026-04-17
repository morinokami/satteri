---
cargo/satteri-mdxjs: patch
cargo/satteri-pulldown-cmark: patch
npm/satteri: patch
---

Fixed unclosed `{` in a paragraph silently consuming later blocks as an MDX expression, and fixed literal `{` inside code spans being falsely reported as an unclosed MDX expression
