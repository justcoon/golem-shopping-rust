// @ts-check

import {
  cleanMojibake,
  escapeHtml,
  formatTime,
  normalizeChunk,
  normalizeRole,
  normalizeToolCall,
  renderMarkdown,
} from "./chat-common.js";

/**
 * Golem Store Manager Web Client
 * Interacts directly with Golem HTTP endpoints for ManagerAgent.
 */

// State
let currentManagerId = localStorage.getItem("golem_store_managerId") || "manager-1";
let isSending = false;
let currentEventSource = null;

// DOM Elements
const managerIdInput = /** @type {HTMLInputElement} */ (document.getElementById("manager-id"));
const messagesContainer = document.getElementById("messages-container");
const welcomeMessage = document.getElementById("welcome-message");
const chatForm = document.getElementById("chat-form");
const messageInput = /** @type {HTMLTextAreaElement} */ (document.getElementById("message-input"));
const sendBtn = /** @type {HTMLButtonElement} */ (document.getElementById("send-btn"));
const clearBtn = /** @type {HTMLButtonElement} */ (document.getElementById("clear-btn"));
const statusBadge = document.getElementById("agent-status");

// Initialize
if (managerIdInput) {
  managerIdInput.value = currentManagerId;

  managerIdInput.addEventListener("change", () => {
    const newId = managerIdInput.value.trim() || "manager-1";
    if (newId !== currentManagerId) {
      currentManagerId = newId;
      localStorage.setItem("golem_store_managerId", currentManagerId);
      loadHistory();
    }
  });
}

// Auto-grow textarea
messageInput.addEventListener("input", () => {
  messageInput.style.height = "auto";
  messageInput.style.height = Math.min(messageInput.scrollHeight, 150) + "px";
});

// Enter to send, Shift+Enter for newline
messageInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    chatForm.dispatchEvent(new Event("submit"));
  }
});

// Handle chips & quick action clicks
document.addEventListener("click", (e) => {
  const target = /** @type {HTMLElement} */ (e.target);
  const prompt =
    target.getAttribute("data-prompt") ||
    target.closest("[data-prompt]")?.getAttribute("data-prompt");
  if (prompt && !isSending) {
    messageInput.value = prompt;
    messageInput.style.height = "auto";
    chatForm.dispatchEvent(new Event("submit"));
  }
});

// Clear history
clearBtn.addEventListener("click", async () => {
  if (isSending) return;
  if (!confirm(`Clear all conversation history for ${currentManagerId}?`)) return;

  try {
    setStatus("busy", "Clearing...");
    const res = await fetch(`/v1/manager/${encodeURIComponent(currentManagerId)}/clear-history`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "{}",
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);

    // Clear messages UI
    messagesContainer.innerHTML = "";
    if (welcomeMessage) {
      messagesContainer.appendChild(welcomeMessage);
      welcomeMessage.style.display = "block";
    }
    setStatus("ready", "Ready");
  } catch (err) {
    console.error("Failed to clear history:", err);
    alert("Failed to clear history: " + err.message);
    setStatus("error", "Error");
  }
});

// Handle form submit
chatForm.addEventListener("submit", async (e) => {
  e.preventDefault();
  const text = messageInput.value.trim();
  if (!text || isSending) return;

  const now = new Date().toISOString();

  // Add user message to UI
  if (welcomeMessage) welcomeMessage.style.display = "none";
  appendUserMessage(text, now);
  messageInput.value = "";
  messageInput.style.height = "auto";

  // Send message
  await sendMessage(text);
});

function setStatus(type, text) {
  if (!statusBadge) return;
  statusBadge.className = `status-badge status-${type}`;
  statusBadge.textContent = text;
}

function appendUserMessage(text, timestamp) {
  const row = document.createElement("div");
  row.className = "message-row user";
  const timeStr = formatTime(timestamp);
  const cleanText = text.trim();
  row.innerHTML = `
    <div class="message-content">
      <div class="message-bubble-header">
        <span class="message-time">${escapeHtml(timeStr)}</span>
        <button class="action-btn reuse-btn" type="button" title="Reuse this message in input">
          <span class="action-icon">🔄</span> Reuse
        </button>
      </div>
      <div class="user-text">${escapeHtml(cleanText)}</div>
    </div>
    <div class="message-avatar">👤</div>
  `;
  messagesContainer.appendChild(row);
  scrollToBottom();

  const reuseBtn = row.querySelector(".reuse-btn");
  if (reuseBtn) {
    reuseBtn.addEventListener("click", () => {
      messageInput.value = cleanText;
      messageInput.style.height = "auto";
      messageInput.style.height = Math.min(messageInput.scrollHeight, 120) + "px";
      messageInput.focus();
      messageInput.scrollIntoView({ behavior: "smooth" });
    });
  }
}

let lastUserMessage = "";

function createAssistantBubble(promptText, timestamp) {
  const row = document.createElement("div");
  row.className = "message-row assistant";
  const timeStr = formatTime(timestamp);
  row.innerHTML = `
    <div class="message-avatar">🛠️</div>
    <div class="message-content">
      <div class="message-bubble-header">
        <span class="message-time">${escapeHtml(timeStr)}</span>
        <div class="message-actions" style="display: none;">
          <button class="action-btn retry-btn" type="button" title="Retry sending this message">
            <span class="action-icon">🔄</span> Retry
          </button>
        </div>
      </div>
      <div class="text-content markdown-body"></div>
      <span class="cursor-blink"></span>
      <details class="tools-details" open style="display: none;">
        <summary class="tools-summary">
          <span class="tools-summary-title">🛠️ Tool Activity (<span class="tools-count">0</span>)</span>
          <span class="tools-toggle-icon">▾</span>
        </summary>
        <div class="tool-cards-container"></div>
      </details>
    </div>
  `;
  messagesContainer.appendChild(row);
  scrollToBottom();

  const toolCardsContainer = row.querySelector(".tool-cards-container");
  const toolsDetails = row.querySelector(".tools-details");
  const toolsCount = row.querySelector(".tools-count");
  const textContent = row.querySelector(".text-content");
  const cursor = row.querySelector(".cursor-blink");
  const messageActions = row.querySelector(".message-actions");
  const retryBtn = row.querySelector(".retry-btn");

  const bubble = {
    row,
    toolCardsContainer,
    toolsDetails,
    toolsCount,
    textContent,
    cursor,
    messageActions,
    retryBtn,
    toolCards: new Map(),
    prompt: promptText || lastUserMessage,
    rawText: "",
    setText(text) {
      bubble.rawText = text || "";
      if (textContent) textContent.innerHTML = renderMarkdown(bubble.rawText);
      scrollToBottom();
    },
    appendText(chunk) {
      bubble.rawText += chunk || "";
      if (textContent) textContent.innerHTML = renderMarkdown(bubble.rawText);
      scrollToBottom();
    },
    updateToolCount() {
      const count = toolCardsContainer.children.length;
      if (toolsCount) toolsCount.textContent = String(count);
      if (toolsDetails) toolsDetails.style.display = count > 0 ? "block" : "none";
    },
    showRetry() {
      row.classList.add("error");
      if (messageActions) messageActions.style.display = "flex";
    },
  };

  if (retryBtn) {
    retryBtn.addEventListener("click", () => {
      if (isSending) return;
      const targetPrompt = bubble.prompt || lastUserMessage;
      if (!targetPrompt) return;
      row.remove();
      sendMessage(targetPrompt);
    });
  }

  return bubble;
}

/**
 * Sends a message via Durable Streams SSE with fallback to synchronous POST.
 * @param {string} message
 */
async function sendMessage(message) {
  lastUserMessage = message;
  isSending = true;
  sendBtn.disabled = true;
  setStatus("busy", "Thinking...");

  const now = new Date().toISOString();
  const bubble = createAssistantBubble(message, now);
  let receivedAnyText = false;

  try {
    // 1. Attempt streaming via Durable Streams (PUT /v1/manager/{id}/chat-stream)
    const sessionRes = await fetch(
      `/v1/manager/${encodeURIComponent(currentManagerId)}/chat-stream`,
      {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
          "Stream-TTL": "300",
        },
        body: JSON.stringify({ message }),
      },
    );

    if (!sessionRes.ok) {
      throw new Error(`Stream session init failed with status ${sessionRes.status}`);
    }

    let sessionPath = sessionRes.headers.get("Location");
    if (!sessionPath) {
      throw new Error("Missing Location header in stream response");
    }

    if (sessionPath.startsWith("http://") || sessionPath.startsWith("https://")) {
      sessionPath = new URL(sessionPath).pathname;
    }

    // Connect EventSource to $result stream
    const slotUrl = `${sessionPath}/streams/%24result?live=sse&offset=-1`;
    await new Promise((resolve, reject) => {
      const es = new EventSource(slotUrl);
      currentEventSource = es;

      es.onmessage = (event) => {
        try {
          if (!event.data) return;
          const rawChunk = JSON.parse(event.data);
          const chunk = normalizeChunk(rawChunk);
          if (!chunk) return;

          if (chunk.tag === "text_delta") {
            receivedAnyText = true;
            bubble.appendText(chunk.delta || "");
          } else if (chunk.tag === "tool_start") {
            const tc = chunk.toolCall;
            if (tc) {
              const card = document.createElement("div");
              card.className = "tool-card running";
              card.innerHTML = `
                <span class="tool-spinner"></span>
                <span>Executing <strong>${escapeHtml(tc.toolName || "tool")}</strong>...</span>
              `;
              bubble.toolCardsContainer.appendChild(card);
              bubble.toolCards.set(tc.id, card);
              bubble.updateToolCount();
              scrollToBottom();
            }
          } else if (chunk.tag === "tool_result") {
            const tc = chunk.toolCall;
            if (tc && bubble.toolCards.has(tc.id)) {
              const card = bubble.toolCards.get(tc.id);
              let summary = "Completed";
              try {
                const resObj = typeof tc.result === "string" ? JSON.parse(tc.result) : tc.result;
                summary = resObj.summary || (resObj.error ? `Failed: ${resObj.error}` : "Done");
              } catch {
                summary = String(tc.result);
              }
              const isFailed = summary.toLowerCase().includes("failed");
              card.className = `tool-card ${isFailed ? "failed" : "completed"}`;
              card.innerHTML = `
                <span>${isFailed ? "⚠️" : "✅"}</span>
                <span><strong>${escapeHtml(tc.toolName || "tool")}:</strong> ${escapeHtml(summary)}</span>
              `;
              scrollToBottom();
            }
          } else if (chunk.tag === "done") {
            es.close();
            currentEventSource = null;
            resolve(undefined);
          } else if (chunk.tag === "error") {
            es.close();
            currentEventSource = null;
            reject(new Error(chunk.error || "Stream returned error"));
          }
        } catch (parseErr) {
          console.warn("Error parsing stream chunk:", parseErr);
        }
      };

      es.onerror = (err) => {
        es.close();
        currentEventSource = null;
        if (receivedAnyText) {
          // Stream interrupted but we received partial output
          resolve(undefined);
        } else {
          reject(err);
        }
      };
    });
  } catch (streamErr) {
    console.warn("Streaming unavailable or interrupted, falling back to sync chat:", streamErr);

    // Fallback: synchronous POST /v1/manager/{managerId}/chat
    try {
      const syncRes = await fetch(`/v1/manager/${encodeURIComponent(currentManagerId)}/chat`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message }),
      });

      if (!syncRes.ok) {
        throw new Error(`Sync chat failed: ${syncRes.status} ${syncRes.statusText}`);
      }

      const syncData = await syncRes.json();
      bubble.setText(syncData.reply || "");

      // Render tool calls if present (handle both toolCalls and tool_calls_used / toolCallsUsed)
      const toolCalls =
        syncData.toolCalls ||
        syncData.tool_calls ||
        syncData.toolCallsUsed ||
        syncData.tool_calls_used;
      if (Array.isArray(toolCalls)) {
        for (const rawTc of toolCalls) {
          const tc = normalizeToolCall(rawTc);
          if (!tc) continue;
          const card = document.createElement("div");
          card.className = "tool-card completed";
          card.innerHTML = `
            <span>✅</span>
            <span><strong>${escapeHtml(tc.toolName)}</strong></span>
          `;
          bubble.toolCardsContainer.appendChild(card);
        }
        bubble.updateToolCount();
      }
    } catch (fallbackErr) {
      console.error("Chat request failed:", fallbackErr);
      bubble.setText(`Error: ${fallbackErr.message}`);
      bubble.showRetry();
    }
  } finally {
    if (bubble.cursor) bubble.cursor.remove();
    if (bubble.row.classList.contains("error")) {
      bubble.showRetry();
    }
    isSending = false;
    sendBtn.disabled = false;
    setStatus(
      bubble.row.classList.contains("error") ? "error" : "ready",
      bubble.row.classList.contains("error") ? "Error" : "Ready",
    );
  }
}

/**
 * Loads conversation history for the current manager.
 */
async function loadHistory() {
  try {
    const res = await fetch(`/v1/manager/${encodeURIComponent(currentManagerId)}/history`);
    if (!res.ok) return;

    let history = await res.json();
    if (history && !Array.isArray(history)) {
      if (Array.isArray(history.value)) history = history.value;
      else if (Array.isArray(history.Ok)) history = history.Ok;
      else if (Array.isArray(history.ok)) history = history.ok;
      else if (Array.isArray(history.messages)) history = history.messages;
    }
    if (!Array.isArray(history) || history.length === 0) {
      if (welcomeMessage) welcomeMessage.style.display = "block";
      return;
    }

    if (welcomeMessage) welcomeMessage.style.display = "none";
    messagesContainer.innerHTML = "";

    let lastPrompt = "";
    for (const msg of history) {
      const role = normalizeRole(msg.role);
      if (role === "user") {
        lastPrompt = msg.content;
        appendUserMessage(msg.content, msg.timestamp);
      } else if (role === "assistant") {
        const bubble = createAssistantBubble(lastPrompt, msg.timestamp);
        bubble.setText(msg.content || "");
        if (bubble.cursor) bubble.cursor.remove();

        const toolCalls = msg.toolCalls || msg.tool_calls || msg.tool_calls_used || msg.toolCallsUsed;
        if (Array.isArray(toolCalls) && toolCalls.length > 0) {
          for (const rawTc of toolCalls) {
            const tc = normalizeToolCall(rawTc);
            if (!tc) continue;
            let summary = "";
            if (tc.result) {
              try {
                const resObj = typeof tc.result === "string" ? JSON.parse(tc.result) : tc.result;
                summary = resObj.summary || (resObj.error ? `Failed: ${resObj.error}` : "");
              } catch {
                summary = String(tc.result);
              }
            }
            const isFailed = summary.toLowerCase().includes("failed") || summary.toLowerCase().includes("error");
            const card = document.createElement("div");
            card.className = `tool-card ${isFailed ? "failed" : "completed"}`;
            card.innerHTML = `
              <span>${isFailed ? "⚠️" : "✅"}</span>
              <span><strong>${escapeHtml(tc.toolName)}</strong>${summary ? `: ${escapeHtml(summary)}` : ""}</span>
            `;
            bubble.toolCardsContainer.appendChild(card);
          }
          bubble.updateToolCount();
          // Collapse historical tools by default to keep chat clean
          if (bubble.toolsDetails) bubble.toolsDetails.open = false;
        }
      }
    }
    scrollToBottom();
  } catch (err) {
    console.warn("Could not load history:", err);
  }
}

function scrollToBottom() {
  messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Initial load
loadHistory();
