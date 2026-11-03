// @ts-check

/**
 * Shared chat formatting and utilities for Golem Web Clients.
 */

/**
 * Escapes HTML characters to prevent XSS.
 * @param {any} str
 * @returns {string}
 */
export function escapeHtml(str) {
  if (!str) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

/**
 * Normalizes mojibake UTF-8 corruption from LLM responses.
 * @param {string} text
 * @returns {string}
 */
export function cleanMojibake(text) {
  if (!text) return "";
  return text
    .replace(/â€“/g, "-")
    .replace(/â€”/g, " - ")
    .replace(/â€˜/g, "'")
    .replace(/â€™/g, "'")
    .replace(/â€œ/g, '"')
    .replace(/â€\x9d/g, '"')
    .replace(/â€/g, '"')
    .replace(/â€¢/g, "•")
    .replace(/â€¦/g, "...")
    .replace(/Â /g, " ")
    .replace(/Â\$/g, "$");
}

/**
 * Formats an ISO timestamp or date into HH:MM.
 * @param {string | undefined | null} [isoString]
 * @returns {string}
 */
export function formatTime(isoString) {
  try {
    const date = isoString ? new Date(isoString) : new Date();
    if (isNaN(date.getTime())) return "";
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return "";
  }
}

/**
 * Renders Markdown formatted text into sanitized HTML.
 * Supports headings, bold/italic, code blocks, inline code, links, blockquotes,
 * and deeply nested unordered/ordered lists.
 * @param {string} md
 * @returns {string}
 */
export function renderMarkdown(md) {
  if (!md) return "";

  md = cleanMojibake(md);

  // 1. Extract code blocks
  /** @type {string[]} */
  const codeBlocks = [];
  let text = md.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, (_, _lang, code) => {
    const placeholder = `@@CODEBLOCK${codeBlocks.length}@@`;
    codeBlocks.push(`<pre class="md-code-block"><code>${escapeHtml(code.trim())}</code></pre>`);
    return `\n\n${placeholder}\n\n`;
  });

  // 2. Extract inline code
  /** @type {string[]} */
  const inlineCodes = [];
  text = text.replace(/`([^`]+)`/g, (_, code) => {
    const placeholder = `@@INLINECODE${inlineCodes.length}@@`;
    inlineCodes.push(`<code class="md-inline-code">${escapeHtml(code)}</code>`);
    return placeholder;
  });

  // 3. Escape HTML
  text = escapeHtml(text);

  // 4. Links: [text](url)
  text = text.replace(
    /\[([^\]]+)\]\(([^)]+)\)/g,
    '<a href="$2" class="md-link" target="_blank" rel="noopener noreferrer">$1</a>',
  );

  // 5. Headings
  text = text.replace(/^### (.*$)/gim, '\n\n<h3 class="md-h3">$1</h3>\n\n');
  text = text.replace(/^## (.*$)/gim, '\n\n<h2 class="md-h2">$1</h2>\n\n');
  text = text.replace(/^# (.*$)/gim, '\n\n<h1 class="md-h1">$1</h1>\n\n');

  // 6. Bold & Italic
  text = text.replace(/\*\*\*(.*?)\*\*\*/g, "<strong><em>$1</em></strong>");
  text = text.replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>");
  text = text.replace(/__(.*?)__/g, "<strong>$1</strong>");
  text = text.replace(/(?<!\*)\*(?!\*)(.*?)(?<!\*)\*(?!\*)/g, "<em>$1</em>");
  text = text.replace(/(?<!_)_(?!_)(.*?)(?<!_)_(?!_)/g, "<em>$1</em>");

  // 7. Unordered, Ordered Lists, and Blockquotes (with nested indentation support)
  const lines = text.split("\n");
  /** @type {string[]} */
  const processed = [];
  let inBq = false;
  /** @type {Array<{ type: "ul" | "ol"; indent: number; hasOpenLi: boolean }>} */
  const listStack = [];

  function closeTopLi() {
    if (listStack.length > 0) {
      const top = listStack[listStack.length - 1];
      if (top.hasOpenLi) {
        if (processed.length > 0 && !processed[processed.length - 1].endsWith("</li>")) {
          processed[processed.length - 1] += "</li>";
        } else {
          processed.push("</li>");
        }
        top.hasOpenLi = false;
      }
    }
  }

  function closeList() {
    if (listStack.length > 0) {
      closeTopLi();
      const popped = listStack.pop();
      if (popped) {
        processed.push(`</${popped.type}>`);
      }
    }
  }

  function closeAllLists() {
    while (listStack.length > 0) {
      closeList();
    }
  }

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const ulMatch = line.match(/^(\s*)[*+-]\s+(.*)$/);
    const olMatch = line.match(/^(\s*)\d+\.\s+(.*)$/);
    const bqMatch = line.match(/^>\s?(.*)$/);

    if (bqMatch) {
      closeAllLists();
      if (!inBq) {
        processed.push("");
        processed.push('<blockquote class="md-blockquote">');
        inBq = true;
      }
      processed.push(`  <p class="md-bq-p">${bqMatch[1]}</p>`);
    } else if (ulMatch || olMatch) {
      if (inBq) {
        processed.push("</blockquote>");
        inBq = false;
      }
      const rawIndent = (ulMatch || olMatch)[1];
      const indent = rawIndent.replace(/\t/g, "  ").length;
      const type = ulMatch ? "ul" : "ol";
      const content = (ulMatch || olMatch)[2];

      if (listStack.length === 0) {
        processed.push("");
        processed.push(`<${type} class="md-${type}">`);
        processed.push(`  <li class="md-li">${content}`);
        listStack.push({ type, indent, hasOpenLi: true });
      } else {
        const top = listStack[listStack.length - 1];
        if (indent > top.indent) {
          processed.push(`<${type} class="md-${type}">`);
          processed.push(`  <li class="md-li">${content}`);
          listStack.push({ type, indent, hasOpenLi: true });
        } else {
          while (listStack.length > 0 && indent < listStack[listStack.length - 1].indent) {
            closeList();
          }
          if (listStack.length === 0) {
            processed.push(`<${type} class="md-${type}">`);
            processed.push(`  <li class="md-li">${content}`);
            listStack.push({ type, indent, hasOpenLi: true });
          } else {
            const currentTop = listStack[listStack.length - 1];
            if (currentTop.type !== type || indent > currentTop.indent) {
              if (currentTop.type !== type) {
                closeList();
              }
              processed.push(`<${type} class="md-${type}">`);
              processed.push(`  <li class="md-li">${content}`);
              listStack.push({ type, indent, hasOpenLi: true });
            } else {
              closeTopLi();
              processed.push(`  <li class="md-li">${content}`);
              currentTop.hasOpenLi = true;
            }
          }
        }
      }
    } else {
      if (inBq) {
        processed.push("</blockquote>");
        processed.push("");
        inBq = false;
      }
      if (listStack.length > 0) {
        closeAllLists();
        processed.push("");
      }
      processed.push(line);
    }
  }

  if (inBq) processed.push("</blockquote>");
  closeAllLists();

  text = processed.join("\n");

  // 8. Paragraphs
  const blocks = text.split(/\n\s*\n/);
  text = blocks
    .map((block) => {
      block = block.trim();
      if (!block) return "";
      if (
        block.startsWith("<ul") ||
        block.startsWith("<ol") ||
        block.startsWith("<blockquote") ||
        block.startsWith("<h1") ||
        block.startsWith("<h2") ||
        block.startsWith("<h3") ||
        block.startsWith("<pre") ||
        block.startsWith("@@CODEBLOCK")
      ) {
        return block;
      }
      return `<p class="md-p">${block.replace(/\n/g, "<br/>")}</p>`;
    })
    .filter(Boolean)
    .join("\n");

  // 9. Restore code blocks & inline code
  text = text.replace(/@@INLINECODE(\d+)@@/g, (_, idx) => inlineCodes[Number(idx)]);
  text = text.replace(/@@CODEBLOCK(\d+)@@/g, (_, idx) => codeBlocks[Number(idx)]);

  return text;
}

/**
 * Normalizes a message role to lowercase ("user", "assistant", "system", etc.).
 * Handles both case-insensitive strings ("User", "Assistant") and object tags.
 * @param {any} role
 * @returns {string}
 */
export function normalizeRole(role) {
  if (!role) return "";
  if (typeof role === "object") {
    const keys = Object.keys(role);
    if (keys.length > 0) return String(keys[0]).toLowerCase().trim();
  }
  return String(role).toLowerCase().trim();
}

/**
 * Normalizes tool call objects across Effect and Rust formats.
 * @param {any} tc
 * @returns {{ id: string, toolName: string, result: any, input: any } | null}
 */
export function normalizeToolCall(tc) {
  if (!tc) return null;
  return {
    id: tc.id || "",
    toolName: tc.toolName || tc.tool_name || "tool",
    result: tc.result !== undefined ? tc.result : "",
    input: tc.input || tc.arguments || "",
  };
}

/**
 * Normalizes SSE stream chunks across Effect and Rust formats.
 * Handles:
 * - Effect: { _tag: "text_delta", content / delta, toolCall, ... }
 * - Rust enum external tag: { "TextDelta": { "content": "..." } }, { "ToolStart": { "tool_call": ... } }, etc.
 * - String chunks: "Done", etc.
 * @param {any} raw
 * @returns {{ tag: string, delta?: string, toolCall?: ReturnType<typeof normalizeToolCall>, error?: string, raw: any } | null}
 */
export function normalizeChunk(raw) {
  if (!raw) return null;

  if (typeof raw === "string") {
    const lower = raw.toLowerCase().trim();
    if (lower === "done" || lower === '"done"') return { tag: "done", raw };
    return { tag: "unknown", raw };
  }

  // Effect-style: { _tag: "text_delta", delta: "...", ... }
  if (raw._tag) {
    const tag = String(raw._tag).toLowerCase();
    return {
      tag,
      delta: raw.delta || raw.content || "",
      toolCall: normalizeToolCall(raw.toolCall || raw.tool_call),
      error: raw.error || "",
      raw,
    };
  }

  // Rust enum external tagging style: { "TextDelta": { "content": "..." } }
  const textObj = raw.TextDelta || raw.textDelta || raw.text_delta;
  if (textObj !== undefined) {
    return {
      tag: "text_delta",
      delta: typeof textObj === "string" ? textObj : textObj.content || textObj.delta || "",
      raw,
    };
  }

  const startObj = raw.ToolStart || raw.toolStart || raw.tool_start;
  if (startObj !== undefined) {
    return {
      tag: "tool_start",
      toolCall: normalizeToolCall(startObj.tool_call || startObj.toolCall || startObj),
      raw,
    };
  }

  const resultObj = raw.ToolResult || raw.toolResult || raw.tool_result;
  if (resultObj !== undefined) {
    return {
      tag: "tool_result",
      toolCall: normalizeToolCall(resultObj.tool_call || resultObj.toolCall || resultObj),
      raw,
    };
  }

  if (raw.Done !== undefined || raw.done !== undefined) {
    return { tag: "done", raw };
  }

  const errObj = raw.Error || raw.error;
  if (errObj !== undefined) {
    return {
      tag: "error",
      error:
        typeof errObj === "string"
          ? errObj
          : errObj.content || errObj.error || JSON.stringify(errObj),
      raw,
    };
  }

  // Generic tag / type field fallback
  const tagStr = String(raw.tag || raw.type || "").toLowerCase();
  if (tagStr.includes("text")) {
    return { tag: "text_delta", delta: raw.content || raw.delta || "", raw };
  }
  if (tagStr.includes("start")) {
    return {
      tag: "tool_start",
      toolCall: normalizeToolCall(raw.toolCall || raw.tool_call),
      raw,
    };
  }
  if (tagStr.includes("result")) {
    return {
      tag: "tool_result",
      toolCall: normalizeToolCall(raw.toolCall || raw.tool_call),
      raw,
    };
  }
  if (tagStr.includes("done")) {
    return { tag: "done", raw };
  }
  if (tagStr.includes("error")) {
    return { tag: "error", error: raw.error || raw.content || "", raw };
  }

  return { tag: "unknown", raw };
}


