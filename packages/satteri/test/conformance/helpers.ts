import { evaluate as mdxEvaluate } from "@mdx-js/mdx";
import { evaluate as satteriEvaluate } from "../../src/index.js";
import { mdxToJs } from "../../src/index.js";
import { renderToStaticMarkup } from "react-dom/server";
import { createElement } from "react";
import * as runtime from "react/jsx-runtime";
import { expect } from "vitest";

function normalizeHtml(html: string): string {
  return html.replace(/>\s+</g, "><").replace(/\s+</g, "<").replace(/>\s+/g, ">").trim();
}

/**
 * Compile the same MDX input through both @mdx-js/mdx and satteri,
 * render with React, and compare the HTML output.
 */
export async function assertMdxConformance(
  input: string,
  components: Record<string, unknown> = {},
): Promise<void> {
  const { default: MdxComponent } = (await mdxEvaluate(input, {
    ...runtime,
  })) as { default: Function };
  const mdxHtml = renderToStaticMarkup(createElement(MdxComponent as any, { components }));

  const { default: SatComponent } = await satteriEvaluate(input, {
    ...runtime,
  });
  const satHtml = renderToStaticMarkup(createElement(SatComponent as any, { components }));

  expect(normalizeHtml(satHtml)).toBe(normalizeHtml(mdxHtml));
}

/**
 * Assert that both compilers reject the input (or both accept it).
 */
export async function assertBothReject(input: string): Promise<void> {
  let mdxOk = true;
  try {
    await mdxEvaluate(input, { ...runtime });
  } catch {
    mdxOk = false;
  }

  let satteriOk = true;
  try {
    mdxToJs(input);
  } catch {
    satteriOk = false;
  }

  expect(satteriOk).toBe(mdxOk);
}

/**
 * Assert that satteri rejects the given input.
 */
export async function assertRejects(input: string): Promise<void> {
  expect(() => mdxToJs(input)).toThrow();
}
