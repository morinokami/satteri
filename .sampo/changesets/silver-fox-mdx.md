---
cargo/satteri-pulldown-cmark: patch
cargo/satteri-ast: patch
npm/satteri: patch
---

Various MDX parsing fixes:

- Fixed non-ASCII content in MDX expressions/JSX inside containers (blockquotes, lists) being corrupted due to byte-by-byte char casting.
- Fixed MDX-only paragraphs inside blockquotes not being unraveled (producing spurious `<p>` wrappers).
- Fixed multiple JSX elements on one line only rendering the first element.
- Multiple other cases of small inconsistencies with `@mdxjs/mdx`, notably in whitespace handling and node positions.
