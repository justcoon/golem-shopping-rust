import test from "node:test";
import assert from "node:assert/strict";
import {
  escapeHtml,
  cleanMojibake,
  formatTime,
  renderMarkdown,
  parseStreamEvents,
  getToolName,
  createRunningToolCard,
  createCompletedToolCard,
  updateCompletedToolCard,
} from "../web/assets/chat-common.js";

test("parseStreamEvents normalizes Golem WIT variant array batches", () => {
  const sseData = JSON.stringify([
    {
      tool_start: {
        content: "Executing search_products...",
        toolCall: {
          id: "call_123",
          toolName: "search_products",
          arguments: '{"query":"iphone"}',
          result: "",
        },
      },
    },
  ]);

  const events = parseStreamEvents(sseData);
  assert.equal(events.length, 1);
  assert.equal(events[0]?.tag, "tool_start");
  assert.equal(events[0]?.payload?.toolCall?.toolName, "search_products");
  assert.equal(events[0]?.payload?.toolCall?.id, "call_123");
});

test("parseStreamEvents normalizes Rust Serde PascalCase variant keys", () => {
  const sseData = JSON.stringify([
    { TextDelta: { content: "Here is what I found" } },
    {
      ToolStart: {
        content: "Executing search_products...",
        tool_call: {
          id: "call_456",
          tool_name: "search_products",
          arguments: '{"query":"shoes"}',
          result: "",
        },
      },
    },
    {
      ToolResult: {
        content: "Found 2 products",
        tool_call: {
          id: "call_456",
          tool_name: "search_products",
          arguments: '{"query":"shoes"}',
          result: '{"summary":"Found 2 items"}',
        },
      },
    },
    { Done: { content: "" } },
  ]);

  const events = parseStreamEvents(sseData);
  assert.equal(events.length, 4);
  assert.equal(events[0]?.tag, "text_delta");
  assert.equal(events[0]?.payload?.content, "Here is what I found");
  assert.equal(events[1]?.tag, "tool_start");
  assert.equal(events[1]?.payload?.tool_call?.tool_name, "search_products");
  assert.equal(events[2]?.tag, "tool_result");
  assert.equal(events[2]?.payload?.tool_call?.id, "call_456");
  assert.equal(events[3]?.tag, "done");
});

test("parseStreamEvents handles tagged format, kebab-case keys, and string chunks", () => {
  const taggedJson = JSON.stringify({ _tag: "text_delta", content: "hello" });
  const taggedEvents = parseStreamEvents(taggedJson);
  assert.equal(taggedEvents.length, 1);
  assert.equal(taggedEvents[0]?.tag, "text_delta");
  assert.equal(taggedEvents[0]?.payload?.content, "hello");

  const kebabJson = JSON.stringify([{ "tool-start": { content: "starting" } }]);
  const kebabEvents = parseStreamEvents(kebabJson);
  assert.equal(kebabEvents.length, 1);
  assert.equal(kebabEvents[0]?.tag, "tool_start");
  assert.equal(kebabEvents[0]?.payload?.content, "starting");

  const doneStr = JSON.stringify("done");
  const doneEvents = parseStreamEvents(doneStr);
  assert.equal(doneEvents.length, 1);
  assert.equal(doneEvents[0]?.tag, "done");
});

test("parseStreamEvents handles empty or malformed input safely", () => {
  assert.deepEqual(parseStreamEvents(""), []);
  assert.deepEqual(parseStreamEvents("invalid-json"), []);
  assert.deepEqual(parseStreamEvents(null), []);
  assert.deepEqual(parseStreamEvents(undefined), []);
});

test("getToolName handles both camelCase and snake_case", () => {
  assert.equal(getToolName({ toolName: "order_lookup" }), "order_lookup");
  assert.equal(getToolName({ tool_name: "product_search" }), "product_search");
  assert.equal(getToolName({}), "tool");
  assert.equal(getToolName(null), "tool");
});

test("renderMarkdown handles code block and inline code placeholders", () => {
  const md = "Here is `const x = 1;` and:\n```javascript\nconsole.log(x);\n```\nDone.";
  const html = renderMarkdown(md);
  assert.match(html, /<code class="md-inline-code">const x = 1;<\/code>/);
  assert.match(html, /<pre class="md-code-block"><code>console.log\(x\);<\/code><\/pre>/);
  assert.doesNotMatch(html, /@@CODEBLOCK/);
  assert.doesNotMatch(html, /@@INLINECODE/);
});

test("renderMarkdown renders order history message correctly", () => {
  const content =
    "You have one previous order:\n\n**Order ID:** 751cf541-2c88-4334-817e-5502e5cef3a0\n**Date:** 2026-11-17\n**Status:** New\n**Items:**\n* Sony WH-1000XM5: $349.00 (1)\n* Apple AirPods Pro (2nd Gen): $229.00 (1)\n**Total:** $578.00";
  const html = renderMarkdown(content);
  assert.match(html, /<strong>Order ID:<\/strong>/);
  assert.match(html, /<ul class="md-ul">/);
  assert.match(html, /Sony WH-1000XM5: \$349\.00/);
  // Ensure lists are not wrapped in paragraphs with rogue <br/> tags
  assert.doesNotMatch(html, /<p[^>]*>(?:(?!<\/p>).)*<ul/s);
  assert.doesNotMatch(html, /<ul[^>]*>\s*<br\s*\/?>/);
  assert.doesNotMatch(html, /<\/li>\s*<br\s*\/?>/);
});

test("createCompletedToolCard parses and summarizes array result", () => {
  const tc = {
    arguments: '{"limit":5}',
    id: "call_7724800839112943212",
    result: '[{"order_id":"751cf541-2c88-4334-817e-5502e5cef3a0"}]',
    tool_name: "get_order_history",
  };
  const card = createCompletedToolCard(tc);
  assert.equal(card.className, "tool-card completed");
  assert.match(card.innerHTML, /get_order_history/);
  assert.match(card.innerHTML, /1 item/);
});
