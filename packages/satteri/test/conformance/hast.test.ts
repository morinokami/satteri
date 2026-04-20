import { describe, test, expect } from "vitest";
import { markdownToHast } from "../../src/index.js";
import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import remarkGfm from "remark-gfm";
import type { Nodes } from "hast";

const processor = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkRehype, { allowDangerousHtml: true });

function referenceHast(md: string): Nodes {
  const mdast = processor.parse(md);
  return processor.runSync(mdast) as Nodes;
}

type AnyNode = Record<string, unknown>;

// Rehype uses the deprecated `align` attribute on table cells; satteri uses
// `style="text-align: ..."` instead. Normalize rehype's `align` to `style`
// so the comparison tests structural equivalence.
function normalizeAlignToStyle(node: AnyNode): AnyNode {
  if (typeof node !== "object" || node === null) return node;
  const out = { ...node };
  if (out.properties && typeof out.properties === "object") {
    const props = { ...(out.properties as Record<string, unknown>) };
    if ("align" in props && typeof props.align === "string") {
      props.style = `text-align: ${props.align}`;
      delete props.align;
    }
    out.properties = props;
  }
  if (Array.isArray(out.children)) {
    out.children = (out.children as AnyNode[]).map(normalizeAlignToStyle);
  }
  return out;
}

function assertHastConformance(md: string): void {
  const satTree = JSON.parse(JSON.stringify(markdownToHast(md)));
  const refTree = normalizeAlignToStyle(JSON.parse(JSON.stringify(referenceHast(md))));
  expect(satTree).toEqual(refTree);
}

describe("HAST conformance: block elements", () => {
  test("heading", () => {
    assertHastConformance("# Hello");
  });

  test("h2", () => {
    assertHastConformance("## World");
  });

  test("paragraph", () => {
    assertHastConformance("hello world");
  });

  test("multiple paragraphs", () => {
    assertHastConformance("first\n\nsecond\n\nthird");
  });

  test("blockquote", () => {
    assertHastConformance("> quoted text");
  });

  test("nested blockquote", () => {
    assertHastConformance("> > nested");
  });

  test("horizontal rule", () => {
    assertHastConformance("---");
  });

  test("code block", () => {
    assertHastConformance("```\ncode\n```");
  });

  test("code block with language", () => {
    assertHastConformance("```js\nconst x = 1\n```");
  });

  test("indented code block", () => {
    assertHastConformance("    indented code");
  });
});

describe("HAST conformance: inline elements", () => {
  test("bold", () => {
    assertHastConformance("**bold**");
  });

  test("italic", () => {
    assertHastConformance("*italic*");
  });

  test("bold and italic", () => {
    assertHastConformance("***bold italic***");
  });

  test("inline code", () => {
    assertHastConformance("`code`");
  });

  test("link", () => {
    assertHastConformance("[text](https://example.com)");
  });

  test("link with title", () => {
    assertHastConformance('[text](https://example.com "title")');
  });

  test("image", () => {
    assertHastConformance("![alt](https://example.com/img.png)");
  });

  test("line break", () => {
    assertHastConformance("line one  \nline two");
  });

  test("mixed inline", () => {
    assertHastConformance("**bold** and *italic* and `code`");
  });
});

describe("HAST conformance: lists", () => {
  test("unordered list", () => {
    assertHastConformance("- one\n- two\n- three");
  });

  test("ordered list", () => {
    assertHastConformance("1. one\n2. two\n3. three");
  });

  test("nested list", () => {
    assertHastConformance("- outer\n  - inner\n- back");
  });

  test("list with paragraphs", () => {
    assertHastConformance("- first\n\n- second");
  });

  test("task list (GFM)", () => {
    assertHastConformance("- [x] done\n- [ ] todo");
  });
});

describe("HAST conformance: tables (GFM)", () => {
  test("simple table", () => {
    assertHastConformance("| a | b |\n| - | - |\n| 1 | 2 |");
  });

  test("table with alignment", () => {
    assertHastConformance("| left | center | right |\n| :--- | :---: | ---: |\n| a | b | c |");
  });
});

describe("HAST conformance: HTML in markdown", () => {
  test("inline HTML", () => {
    assertHastConformance("hello <em>world</em>");
  });

  test("block HTML", () => {
    assertHastConformance("<div>block</div>");
  });
});

describe("HAST conformance: edge cases", () => {
  test("empty input", () => {
    assertHastConformance("");
  });

  test("only whitespace", () => {
    assertHastConformance("   ");
  });

  test("escaped characters", () => {
    assertHastConformance("\\*not bold\\*");
  });

  test("autolink", () => {
    assertHastConformance("<https://example.com>");
  });

  test("GFM strikethrough", () => {
    assertHastConformance("~~deleted~~");
  });

  test("heading with inline formatting", () => {
    assertHastConformance("## The `config` object");
  });

  test("blockquote with formatting", () => {
    assertHastConformance("> **bold** in quote");
  });

  test("reference link", () => {
    assertHastConformance("[text][ref]\n\n[ref]: https://example.com");
  });
});
