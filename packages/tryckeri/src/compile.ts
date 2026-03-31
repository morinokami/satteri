/**
 * Top-level compile functions — the primary public API.
 *
 * The pipeline keeps the HAST arena in Rust memory via opaque handles.
 * Only matched nodes and mutation commands cross the NAPI boundary.
 */

import {
  visitHast,
  visitHastHandle,
  resolveSubscriptions,
  type HastHandle,
} from "./hast/hast-visitor.js";
import { HastReader } from "./hast/hast-reader.js";
import { DataMap } from "./data-map.js";
import { runPluginsOnBuffer, ProcessorContext } from "./pipeline.js";
import type { MdastPluginDefinition, HastPluginDefinition } from "./plugin.js";
import {
  parseToHtml,
  compileMdx,
  createHastHandle,
  createMdxHastHandle,
  createHastHandleFromBuffer,
  renderHandle,
  compileHandle,
  serializeHandle,
  applyMutations,
  applyCommandsToHandle,
  parseToBuffer,
  parseMdxToBuffer,
  mdastBufferToHastBuffer,
  applyMutationsAndConvertToHast,
} from "../index.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function initPlugins<T>(
  plugins: { name: string; createOnce(ctx: ProcessorContext): T }[],
): { instance: T; name: string }[] {
  const ctx = new ProcessorContext();
  return plugins.map((def) => ({
    instance: def.createOnce(ctx),
    name: def.name,
  }));
}

function extractBuffer(result: { buffer: ArrayBuffer | Uint8Array }): Uint8Array {
  return result.buffer instanceof Uint8Array ? result.buffer : new Uint8Array(result.buffer);
}

// ---------------------------------------------------------------------------
// HAST plugin runner (handle-based, arena stays in Rust)
// ---------------------------------------------------------------------------

function runHastPluginsOnHandle(handle: HastHandle, plugins: HastPluginDefinition[]): void {
  if (plugins.length === 0) return;

  const instances = initPlugins(plugins);
  for (const { instance } of instances) {
    const subs = resolveSubscriptions(instance);
    if (subs) {
      // Handle path: Rust walks, only matched nodes cross the boundary
      visitHastHandle(handle, instance, subs);
    } else {
      // Buffer fallback: transformRoot or bare function plugins
      const buf = serializeHandle(handle);
      const result = visitHast(new HastReader(buf), instance, new DataMap());
      if (result.hasMutations) {
        applyCommandsToHandle(handle, result.commandBuffer);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Configuration for static subtree collapsing during MDX compilation. */
export interface OptimizeStaticConfig {
  component: string;
  prop: string;
  wrapPropValue?: boolean;
  ignoreElements?: string[];
}

export interface CompileOptions {
  mdastPlugins?: MdastPluginDefinition[];
  hastPlugins?: HastPluginDefinition[];
  optimizeStatic?: OptimizeStaticConfig;
}

export function compileMarkdownToHtml(source: string, options: CompileOptions = {}): string {
  const { mdastPlugins = [], hastPlugins = [] } = options;

  // Fast path: no plugins
  if (mdastPlugins.length === 0 && hastPlugins.length === 0) {
    return parseToHtml(source);
  }

  // Create HAST handle (arena stays in Rust)
  let handle: HastHandle;
  if (mdastPlugins.length > 0) {
    handle = runMdastThenCreateHandle(source, mdastPlugins, false);
  } else {
    handle = createHastHandle(source);
  }

  // Run HAST plugins on the handle
  runHastPluginsOnHandle(handle, hastPlugins);

  // Render directly from handle — no buffer copy
  return renderHandle(handle);
}

export function compileMdxToJs(source: string, options: CompileOptions = {}): string {
  const { mdastPlugins = [], hastPlugins = [], optimizeStatic } = options;
  const mdxOptions = optimizeStatic ? { optimizeStatic } : undefined;

  // Fast path: no plugins
  if (mdastPlugins.length === 0 && hastPlugins.length === 0) {
    return compileMdx(source, mdxOptions);
  }

  // Create HAST handle
  let handle: HastHandle;
  if (mdastPlugins.length > 0) {
    handle = runMdastThenCreateHandle(source, mdastPlugins, true);
  } else {
    handle = createMdxHastHandle(source);
  }

  runHastPluginsOnHandle(handle, hastPlugins);

  return compileHandle(handle, mdxOptions);
}

// ---------------------------------------------------------------------------
// MDAST plugins → HAST handle
// ---------------------------------------------------------------------------

function runMdastThenCreateHandle(
  source: string,
  mdastPlugins: MdastPluginDefinition[],
  mdx: boolean,
): HastHandle {
  const mdastBuf = mdx ? parseMdxToBuffer(source) : parseToBuffer(source);
  const instances = initPlugins(mdastPlugins);
  const result = runPluginsOnBuffer(mdastBuf, instances, { deferLast: true });

  let hastBuf: Uint8Array;
  if (result.pendingCommands) {
    hastBuf = applyMutationsAndConvertToHast(extractBuffer(result), result.pendingCommands);
  } else {
    hastBuf = mdastBufferToHastBuffer(extractBuffer(result));
  }

  return createHastHandleFromBuffer(hastBuf);
}
