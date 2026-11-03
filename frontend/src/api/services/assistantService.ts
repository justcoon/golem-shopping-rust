import apiClient from "../config";
import { getProductsByIds, type Product } from "./productService.ts";
import type { PriceFilterOptions } from "@/api/services/pricingService.ts";

export interface RecommendedItems {
  product_ids: string[];
  product_brands: string[];
}

export interface ChatThreadSummary {
  chat_id: string;
  title: string;
  created_at: string;
}

export interface ToolCallRecord {
  id: string;
  tool_name: string;
  arguments: string;
  result: string;
}

export interface ChatMessage {
  role: string;
  content: string;
  tool_calls?: ToolCallRecord[];
  timestamp: string;
  type?: string;
}

export interface TextDeltaChunk {
  _tag: "text_delta";
  content: string;
}

export interface ToolStartChunk {
  _tag: "tool_start";
  content: string;
  tool_call: ToolCallRecord;
}

export interface ToolResultChunk {
  _tag: "tool_result";
  content: string;
  tool_call: ToolCallRecord;
}

export interface DoneChunk {
  _tag: "done";
  content: string;
}

export interface ErrorChunk {
  _tag: "error";
  content: string;
}

export type ChatChunk =
  | TextDeltaChunk
  | ToolStartChunk
  | ToolResultChunk
  | DoneChunk
  | ErrorChunk;

export function normalizeChatChunk(raw: any): ChatChunk | null {
  if (!raw || typeof raw !== "object") return null;

  if (typeof raw._tag === "string") {
    return raw as ChatChunk;
  }

  const keys = Object.keys(raw);
  if (keys.length === 1) {
    const key = keys[0];
    const lowerKey = key.toLowerCase().replace(/_/g, "");
    const payload = raw[key] ?? {};

    if (lowerKey === "textdelta") {
      return {
        _tag: "text_delta",
        content: String(payload.content ?? (typeof payload === "string" ? payload : "")),
      };
    } else if (lowerKey === "toolstart") {
      const tc = payload.tool_call ?? payload;
      return {
        _tag: "tool_start",
        content: String(payload.content ?? ""),
        tool_call: {
          id: tc.id ?? "",
          tool_name: tc.tool_name ?? "",
          arguments: typeof tc.arguments === "string" ? tc.arguments : JSON.stringify(tc.arguments ?? ""),
          result: typeof tc.result === "string" ? tc.result : JSON.stringify(tc.result ?? ""),
        },
      };
    } else if (lowerKey === "toolresult") {
      const tc = payload.tool_call ?? payload;
      return {
        _tag: "tool_result",
        content: String(payload.content ?? ""),
        tool_call: {
          id: tc.id ?? "",
          tool_name: tc.tool_name ?? "",
          arguments: typeof tc.arguments === "string" ? tc.arguments : JSON.stringify(tc.arguments ?? ""),
          result: typeof tc.result === "string" ? tc.result : JSON.stringify(tc.result ?? ""),
        },
      };
    } else if (lowerKey === "done") {
      return {
        _tag: "done",
        content: String(payload.content ?? (typeof payload === "string" ? payload : "")),
      };
    } else if (lowerKey === "error") {
      return {
        _tag: "error",
        content: String(payload.content ?? (typeof payload === "string" ? payload : "")),
      };
    }
  }

  return null;
}

export interface ChatResponse {
  reply: string;
  tool_calls_used: ToolCallRecord[];
  type?: string;
}

export const getRecommendedProducts = async (
  userId: string,
  options?: PriceFilterOptions,
): Promise<Product[]> => {
  try {
    const recommended: RecommendedItems = await apiClient.get(
      `/v1/assistant/${userId}/recommended-items`,
    );
    return await getProductsByIds(recommended.product_ids || [], options);
  } catch (error) {
    console.error("Failed to fetch recommended products:", error);
    throw error;
  }
};

export const listAssistantChats = async (userId: string): Promise<ChatThreadSummary[]> => {
  try {
    return await apiClient.get(`/v1/assistant/${userId}/chats`);
  } catch (error) {
    console.error("Failed to list assistant chats:", error);
    throw error;
  }
};

export const createAssistantChat = async (
  userId: string,
  title?: string,
): Promise<ChatThreadSummary> => {
  try {
    return await apiClient.post(`/v1/assistant/${userId}/chats`, { title });
  } catch (error) {
    console.error("Failed to create assistant chat:", error);
    throw error;
  }
};

export const deleteAssistantChat = async (userId: string, chatId: string): Promise<boolean> => {
  try {
    return await apiClient.delete(`/v1/assistant/${userId}/chats/${chatId}`);
  } catch (error) {
    console.error("Failed to delete assistant chat:", error);
    throw error;
  }
};

export const chatWithAssistant = async (
  userId: string,
  message: string,
  chatId: string = "default",
): Promise<ChatResponse> => {
  try {
    return await apiClient.post(`/v1/assistant/${userId}/chats/${chatId}/chat`, { message });
  } catch (error) {
    console.error("Failed to send chat message:", error);
    throw error;
  }
};

export const getAssistantHistory = async (
  userId: string,
  chatId: string = "default",
): Promise<ChatMessage[]> => {
  try {
    const rawList = await apiClient.get<any[]>(`/v1/assistant/${userId}/chats/${chatId}/history`);
    return (rawList || []).map((msg) => ({
      role: String(msg.role || "").toLowerCase().trim(),
      content: msg.content || "",
      timestamp: msg.timestamp || "",
      type: String(msg.type || msg["r#type"] || "ok").toLowerCase().trim(),
      tool_calls: msg.tool_calls ?? msg.toolCalls,
    }));
  } catch (error) {
    console.error("Failed to fetch assistant history:", error);
    throw error;
  }
};

export const clearAssistantHistory = async (
  userId: string,
  chatId: string = "default",
): Promise<void> => {
  try {
    await apiClient.post(`/v1/assistant/${userId}/chats/${chatId}/clear-history`, {});
  } catch (error) {
    console.error("Failed to clear assistant history:", error);
    throw error;
  }
};

export interface ChatSession {
  sessionPath: string;
  sendMessage: (message: string) => Promise<void>;
  close: () => void;
}

export const createChatSession = async (
  userId: string,
  onChunk: (chunk: ChatChunk) => void,
  onError?: (error: any) => void,
  signal?: AbortSignal,
  chatId: string = "default",
): Promise<ChatSession> => {
  if (signal?.aborted) {
    throw new Error("Chat session creation aborted");
  }

  const putResponse = await fetch(`/api/v1/assistant/${userId}/chats/${chatId}/chat-session`, {
    method: "PUT",
    signal,
    headers: {
      "Content-Type": "application/json",
      "Stream-TTL": "600",
    },
    body: JSON.stringify({}),
  });

  if (!putResponse.ok) {
    throw new Error(
      `Failed to create bi-directional chat session: ${putResponse.status} ${putResponse.statusText}`,
    );
  }

  const location = putResponse.headers.get("Location");
  if (!location) {
    throw new Error("Missing Location header from chat session creation");
  }

  let sessionPath = location;
  try {
    if (sessionPath.startsWith("http://") || sessionPath.startsWith("https://")) {
      sessionPath = new URL(sessionPath).pathname;
    }
  } catch {}
  if (!sessionPath.startsWith("/api")) {
    sessionPath = `/api${sessionPath.startsWith("/") ? "" : "/"}${sessionPath}`;
  }

  const outboxUrl = `${sessionPath}/streams/outbox?live=sse&offset=-1`;
  const eventSource = new EventSource(outboxUrl);
  let isClosed = false;

  const close = () => {
    if (isClosed) return;
    isClosed = true;
    eventSource.close();
    fetch(sessionPath, { method: "DELETE" }).catch(() => {});
  };

  signal?.addEventListener("abort", () => {
    close();
  });

  const handleData = (event: MessageEvent) => {
    try {
      if (!event.data) return;
      const data = JSON.parse(event.data);
      const processItem = (item: any) => {
        const chunk = normalizeChatChunk(item);
        if (chunk) {
          onChunk(chunk);
        }
      };

      if (Array.isArray(data)) {
        for (const item of data) {
          processItem(item);
        }
      } else if (data && typeof data === "object") {
        processItem(data);
      }
    } catch (err) {
      console.warn("Failed to parse chat session SSE chunk:", err);
    }
  };

  eventSource.addEventListener("data", handleData as EventListener);
  eventSource.addEventListener("message", handleData as EventListener);
  eventSource.onmessage = handleData;

  eventSource.onerror = (err) => {
    if (!isClosed && onError) {
      onError(err);
    }
  };

  const sendMessage = async (message: string): Promise<void> => {
    if (isClosed) {
      throw new Error("Chat session is closed");
    }
    const inboxUrl = `${sessionPath}/streams/inbox`;
    const res = await fetch(inboxUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ message }),
    });
    if (!res.ok) {
      throw new Error(`Failed to post message to inbox: ${res.status} ${res.statusText}`);
    }
  };

  return {
    sessionPath,
    sendMessage,
    close,
  };
};
