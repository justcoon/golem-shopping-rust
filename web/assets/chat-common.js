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
        if (listStack.length === 0) {
          processed.push("");
        }
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
        processed.push("");
        inBq = false;
      }

      const indent = (ulMatch ? ulMatch[1] : olMatch[1]).length;
      const content = ulMatch ? ulMatch[2] : olMatch[2];
      const type = ulMatch ? "ul" : "ol";

      if (listStack.length === 0) {
        if (processed.length > 0 && processed[processed.length - 1] !== "") {
          processed.push("");
        }
        processed.push(`<${type} class="md-${type}">`);
        processed.push(`  <li class="md-li">${content}`);
        listStack.push({ type, indent, hasOpenLi: true });
      } else {
        const currentTop = listStack[listStack.length - 1];
        if (indent > currentTop.indent) {
          // Open nested list
          processed.push(`<${type} class="md-${type}">`);
          processed.push(`  <li class="md-li">${content}`);
          listStack.push({ type, indent, hasOpenLi: true });
        } else if (indent === currentTop.indent) {
          if (currentTop.type === type) {
            closeTopLi();
            processed.push(`  <li class="md-li">${content}`);
            currentTop.hasOpenLi = true;
          } else {
            closeList();
            processed.push(`<${type} class="md-${type}">`);
            processed.push(`  <li class="md-li">${content}`);
            listStack.push({ type, indent, hasOpenLi: true });
          }
        } else {
          // Unwind to matching or lower indent
          while (listStack.length > 0 && listStack[listStack.length - 1].indent > indent) {
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
 * Normalizes raw Golem SSE event data into typed chat stream events.
 * Handles array batches, WIT variant keying ({ tool_start: {...} }),
 * kebab-case keys ({ "tool-start": {...} }), PascalCase ({ "ToolStart": {...} }),
 * and Effect TaggedStruct ({ _tag: "tool_start", ... }).
 *
 * @param {string} rawData
 * @returns {Array<{ tag: string, payload: any }>}
 */
export function parseStreamEvents(rawData) {
  if (!rawData) return [];
  try {
    const parsed = JSON.parse(rawData);
    const items = Array.isArray(parsed) ? parsed : [parsed];
    /** @type {Array<{ tag: string, payload: any }>} */
    const events = [];

    for (const item of items) {
      if (typeof item === "string") {
        const lower = item.toLowerCase().trim();
        if (lower === "done" || lower === '"done"') {
          events.push({ tag: "done", payload: { content: "" } });
        }
        continue;
      }

      if (!item || typeof item !== "object") continue;

      let tag = item._tag;
      let payload = item;

      if (!tag) {
        const keys = Object.keys(item);
        if (keys.length === 1) {
          const key = keys[0];
          tag = key
            .replace(/([a-z])([A-Z])/g, "$1_$2")
            .replace(/-/g, "_")
            .toLowerCase();
          payload = item[key] || {};
        }
      } else {
        tag = String(tag)
          .replace(/([a-z])([A-Z])/g, "$1_$2")
          .replace(/-/g, "_")
          .toLowerCase();
      }

      if (tag) {
        events.push({ tag, payload });
      }
    }

    return events;
  } catch (err) {
    console.warn("Failed to parse stream event:", err);
    return [];
  }
}

/**
 * Extracts a normalized tool name from a tool call object.
 * @param {any} toolCall
 * @returns {string}
 */
export function getToolName(toolCall) {
  return toolCall?.toolName || toolCall?.tool_name || "tool";
}

/**
 * Creates a running tool card element with a spinner.
 * @param {any} toolCall
 * @returns {any}
 */
export function createRunningToolCard(toolCall) {
  const card =
    typeof document !== "undefined"
      ? document.createElement("div")
      : { className: "", innerHTML: "" };
  card.className = "tool-card running";
  card.innerHTML = `
    <span class="tool-spinner"></span>
    <span>Executing <strong>${escapeHtml(getToolName(toolCall))}</strong>...</span>
  `;
  return card;
}

/**
 * Updates a running tool card element to completed or failed state.
 * @param {any} card
 * @param {any} toolCall
 */
export function updateCompletedToolCard(card, toolCall) {
  let summary = "Completed";
  try {
    const resObj =
      typeof toolCall.result === "string" ? JSON.parse(toolCall.result) : toolCall.result;
    if (Array.isArray(resObj)) {
      summary = `${resObj.length} item${resObj.length === 1 ? "" : "s"}`;
    } else {
      summary = resObj?.summary || (resObj?.error ? `Failed: ${resObj.error}` : "Done");
    }
  } catch {
    summary = toolCall?.result ? String(toolCall.result) : "Done";
  }
  const isFailed = summary.toLowerCase().includes("failed");
  card.className = `tool-card ${isFailed ? "failed" : "completed"}`;
  card.innerHTML = `
    <span>${isFailed ? "⚠️" : "✅"}</span>
    <span><strong>${escapeHtml(getToolName(toolCall))}:</strong> ${escapeHtml(summary)}</span>
  `;
}

/**
 * Creates a static completed tool card (for historical messages or sync fallback).
 * @param {any} toolCall
 * @returns {any}
 */
export function createCompletedToolCard(toolCall) {
  const card =
    typeof document !== "undefined"
      ? document.createElement("div")
      : { className: "", innerHTML: "" };
  let summary = "";
  if (toolCall?.result) {
    try {
      const resObj =
        typeof toolCall.result === "string" ? JSON.parse(toolCall.result) : toolCall.result;
      if (Array.isArray(resObj)) {
        summary = `${resObj.length} item${resObj.length === 1 ? "" : "s"}`;
      } else {
        summary = resObj?.summary || (resObj?.error ? `Failed: ${resObj.error}` : "");
      }
    } catch {
      summary = "";
    }
  }
  const isFailed =
    summary.toLowerCase().includes("failed") || summary.toLowerCase().includes("error");
  card.className = `tool-card ${isFailed ? "failed" : "completed"}`;
  card.innerHTML = `
    <span>${isFailed ? "⚠️" : "✅"}</span>
    <span><strong>${escapeHtml(getToolName(toolCall))}</strong>${summary ? `: ${escapeHtml(summary)}` : ""}</span>
  `;
  return card;
}
