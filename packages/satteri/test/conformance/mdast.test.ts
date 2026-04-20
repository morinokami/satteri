import { describe, test, expect } from "vitest";
import { markdownToMdast } from "../../src/index.js";
import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";

const processor = unified().use(remarkParse).use(remarkGfm);

function referenceMdast(md: string): unknown {
  return processor.parse(md);
}

function assertMdastConformance(md: string): void {
  const satTree = JSON.parse(JSON.stringify(markdownToMdast(md)));
  const refTree = JSON.parse(JSON.stringify(referenceMdast(md)));
  expect(satTree).toEqual(refTree);
}

describe("MDAST conformance: block elements", () => {
  test("heading", () => {
    assertMdastConformance("# Hello");
  });

  test("h2", () => {
    assertMdastConformance("## World");
  });

  test("paragraph", () => {
    assertMdastConformance("hello world");
  });

  test("multiple paragraphs", () => {
    assertMdastConformance("first\n\nsecond\n\nthird");
  });

  test("blockquote", () => {
    assertMdastConformance("> quoted text");
  });

  test("nested blockquote", () => {
    assertMdastConformance("> > nested");
  });

  test("horizontal rule", () => {
    assertMdastConformance("---");
  });

  test("code block", () => {
    assertMdastConformance("```\ncode\n```");
  });

  test("code block with language", () => {
    assertMdastConformance("```js\nconst x = 1\n```");
  });

  test("indented code block", () => {
    assertMdastConformance("    indented code");
  });
});

describe("MDAST conformance: inline elements", () => {
  test("bold", () => {
    assertMdastConformance("**bold**");
  });

  test("italic", () => {
    assertMdastConformance("*italic*");
  });

  test("bold and italic", () => {
    assertMdastConformance("***bold italic***");
  });

  test("inline code", () => {
    assertMdastConformance("`code`");
  });

  test("link", () => {
    assertMdastConformance("[text](https://example.com)");
  });

  test("link with title", () => {
    assertMdastConformance('[text](https://example.com "title")');
  });

  test("image", () => {
    assertMdastConformance("![alt](https://example.com/img.png)");
  });

  test("line break", () => {
    assertMdastConformance("line one  \nline two");
  });

  test("mixed inline", () => {
    assertMdastConformance("**bold** and *italic* and `code`");
  });
});

describe("MDAST conformance: lists", () => {
  test("unordered list", () => {
    assertMdastConformance("- one\n- two\n- three");
  });

  test("ordered list", () => {
    assertMdastConformance("1. one\n2. two\n3. three");
  });

  test("nested list", () => {
    assertMdastConformance("- outer\n  - inner\n- back");
  });

  test("list with paragraphs (loose)", () => {
    assertMdastConformance("- first\n\n- second");
  });

  test("task list (GFM)", () => {
    assertMdastConformance("- [x] done\n- [ ] todo");
  });
});

describe("MDAST conformance: tables (GFM)", () => {
  test("simple table", () => {
    assertMdastConformance("| a | b |\n| - | - |\n| 1 | 2 |");
  });

  test("table with alignment", () => {
    assertMdastConformance("| left | center | right |\n| :--- | :---: | ---: |\n| a | b | c |");
  });
});

describe("MDAST conformance: HTML in markdown", () => {
  test("inline HTML", () => {
    assertMdastConformance("hello <em>world</em>");
  });

  test("block HTML", () => {
    assertMdastConformance("<div>block</div>");
  });
});

describe("MDAST conformance: edge cases", () => {
  test("empty input", () => {
    assertMdastConformance("");
  });

  test("only whitespace", () => {
    assertMdastConformance("   ");
  });

  test("escaped characters", () => {
    assertMdastConformance("\\*not bold\\*");
  });

  test("autolink", () => {
    assertMdastConformance("<https://example.com>");
  });

  test("GFM strikethrough", () => {
    assertMdastConformance("~~deleted~~");
  });

  test("heading with inline formatting", () => {
    assertMdastConformance("## The `config` object");
  });

  test("blockquote with formatting", () => {
    assertMdastConformance("> **bold** in quote");
  });

  test.skip("reference link", () => {
    // Satteri resolves references eagerly (produces `link` node),
    // remark keeps them as `linkReference` + `definition`. To be changed.
    assertMdastConformance("[text][ref]\n\n[ref]: https://example.com");
  });
});
