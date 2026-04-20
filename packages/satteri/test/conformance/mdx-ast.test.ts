import { describe, test, expect } from "vitest";
import { remark } from "remark";
import remarkMdx from "remark-mdx";
import { toHast } from "mdast-util-to-hast";
import { mdxToMdast, mdxToHast } from "../../src/index.js";

const mdxParser = remark().use(remarkMdx);

const MDX_PASS_THROUGH_NODES = [
  "mdxJsxFlowElement",
  "mdxJsxTextElement",
  "mdxFlowExpression",
  "mdxTextExpression",
  "mdxjsEsm",
];

type AnyNode = Record<string, unknown>;

function stripPositionsAndEstree(node: unknown): unknown {
  if (typeof node !== "object" || node === null) return node;
  if (Array.isArray(node)) return node.map(stripPositionsAndEstree);
  const out: AnyNode = {};
  for (const [k, v] of Object.entries(node as AnyNode)) {
    if (k === "position") continue;
    // remark-mdx includes parsed estree in `data`; satteri doesn't
    if (k === "data") continue;
    if (Array.isArray(v)) out[k] = v.map(stripPositionsAndEstree);
    else if (typeof v === "object" && v !== null)
      out[k] = stripPositionsAndEstree(v);
    else out[k] = v;
  }
  return out;
}

function referenceMdast(input: string): unknown {
  return stripPositionsAndEstree(mdxParser.parse(input));
}

function satteriMdast(input: string): unknown {
  return stripPositionsAndEstree(mdxToMdast(input));
}

function assertMdastConformance(input: string): void {
  const sat = satteriMdast(input);
  const ref = referenceMdast(input);
  expect(sat).toEqual(ref);
}

function referenceHast(input: string): unknown {
  const mdast = mdxParser.parse(input);
  return stripPositionsAndEstree(
    toHast(mdast, { allowDangerousHtml: true, passThrough: MDX_PASS_THROUGH_NODES }),
  );
}

function satteriHastTree(input: string): unknown {
  return stripPositionsAndEstree(mdxToHast(input));
}

function assertHastConformance(input: string): void {
  const sat = satteriHastTree(input);
  const ref = referenceHast(input);
  expect(sat).toEqual(ref);
}

describe("MDX MDAST conformance", () => {
  test("self-closing flow element", () => {
    assertMdastConformance("<Foo bar={1}/>\n");
  });

  test("flow element with children", () => {
    assertMdastConformance("<Box>hello</Box>\n");
  });

  test("inline JSX in paragraph", () => {
    assertMdastConformance("hello <Foo/> world\n");
  });

  test("fragment", () => {
    assertMdastConformance("<>hello</>\n");
  });

  test("flow expression", () => {
    assertMdastConformance("{1 + 2}\n");
  });

  test("inline expression", () => {
    assertMdastConformance("result: {1 + 2}\n");
  });

  test("multiple self-closing on one line", () => {
    assertMdastConformance("<Foo bar={1}/><Bar baz={2}/>\n");
  });

  test("balanced open/close", () => {
    assertMdastConformance("<a></a>\n");
  });

  test("ESM import", () => {
    assertMdastConformance('import Foo from "foo"\n');
  });

  test("ESM export", () => {
    assertMdastConformance("export const x = 42\n");
  });

  test("boolean attribute", () => {
    assertMdastConformance("<Foo disabled/>\n");
  });

  test("string attribute", () => {
    assertMdastConformance('<Foo label="hello"/>\n');
  });

  test("expression attribute", () => {
    assertMdastConformance("<Foo bar={1 + 2}/>\n");
  });

  test("spread attribute", () => {
    assertMdastConformance("<Foo {...props}/>\n");
  });

  test("JSX with expression child", () => {
    assertMdastConformance("<Box>{1 + 2}</Box>\n");
  });

  test("nested JSX", () => {
    assertMdastConformance("<Box><Foo/></Box>\n");
  });

  test("paragraph with expression and text", () => {
    assertMdastConformance("a {1} b\n");
  });

  test("heading with JSX", () => {
    assertMdastConformance("# <Foo/>\n");
  });

  test("blockquote with expression", () => {
    assertMdastConformance("> {1 + 2}\n");
  });

  test("list item with JSX", () => {
    assertMdastConformance("- <Foo/>\n");
  });
});

describe("MDX HAST conformance", () => {
  test("self-closing flow element", () => {
    assertHastConformance("<Foo bar={1}/>\n");
  });

  // toHast with passThrough doesn't do MDX paragraph unraveling, but
  // @mdx-js/mdx does (no <p> wrapper for JSX-only paragraphs). Sätteri
  // matches @mdx-js/mdx behavior here, which the HTML conformance tests verify.
  test("flow element with children (unraveled)", () => {
    const sat = satteriHastTree("<Box>hello</Box>\n") as any;
    // Should be unraveled: root > mdxJsxTextElement (no <p> wrapper)
    expect(sat.children[0].type).toBe("mdxJsxTextElement");
    expect(sat.children[0].name).toBe("Box");
  });

  test("inline JSX in paragraph", () => {
    assertHastConformance("hello <Foo/> world\n");
  });

  test("flow expression", () => {
    assertHastConformance("{1 + 2}\n");
  });

  test("inline expression", () => {
    assertHastConformance("result: {1 + 2}\n");
  });

  test("ESM import", () => {
    assertHastConformance('import Foo from "foo"\n');
  });

  test("ESM export", () => {
    assertHastConformance("export const x = 42\n");
  });

  test("heading with JSX", () => {
    assertHastConformance("# <Foo/>\n");
  });

  test("blockquote with expression", () => {
    assertHastConformance("> {1 + 2}\n");
  });

  test("markdown paragraph with JSX and text", () => {
    assertHastConformance("hello <Foo/> world\n");
  });
});
