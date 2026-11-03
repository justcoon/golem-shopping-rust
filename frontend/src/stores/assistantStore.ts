import { defineStore } from "pinia";
import { ref, computed } from "vue";
import {
  getRecommendedProducts,
  chatWithAssistant,
  createChatSession,
  getAssistantHistory,
  clearAssistantHistory,
  listAssistantChats,
  createAssistantChat,
  deleteAssistantChat,
  type ChatSession,
  type ChatMessage,
  type ToolCallRecord,
  type ChatChunk,
  type ChatThreadSummary,
} from "@/api/services/assistantService";
import { useAuthStore } from "./authStore";
import { useCartStore } from "./cartStore";
import type { PriceFilterOptions } from "@/api/services/pricingService.ts";
import type { Product } from "@/api/services/productService.ts";

export interface ActiveToolStatus {
  id: string;
  name: string;
  status: "running" | "completed" | "error";
  summary?: string;
}

export const useAssistantStore = defineStore("assistant", () => {
  const recommendedProducts = ref<Product[]>([]);
  const messages = ref<ChatMessage[]>([]);
  const chats = ref<ChatThreadSummary[]>([]);
  const currentChatId = ref<string>("default");
  const isLoading = ref(false);
  const isStreaming = ref(false);
  const activeTools = ref<ActiveToolStatus[]>([]);
  const streamingText = ref("");
  const error = ref<Error | null>(null);
  const lastUpdated = ref<Date | null>(null);
  const activeSession = ref<ChatSession | null>(null);

  const authStore = useAuthStore();
  const cartStore = useCartStore();

  const hasRecommendations = computed(() => recommendedProducts.value.length > 0);
  const hasMessages = computed(() => messages.value.length > 0);

  const fetchChats = async () => {
    if (!authStore.userId) return;
    try {
      chats.value = await listAssistantChats(authStore.userId);
    } catch (err) {
      console.warn("Could not fetch assistant chats:", err);
    }
  };

  const fetchHistory = async () => {
    if (!authStore.userId) return;
    try {
      messages.value = await getAssistantHistory(authStore.userId, currentChatId.value);
    } catch (err) {
      console.warn("Could not fetch assistant history:", err);
    }
  };

  const closeSession = () => {
    if (activeSession.value) {
      activeSession.value.close();
      activeSession.value = null;
    }
  };

  const switchChat = async (chatId: string) => {
    if (chatId === currentChatId.value) return;
    closeSession();
    currentChatId.value = chatId;
    messages.value = [];
    activeTools.value = [];
    streamingText.value = "";
    await fetchHistory();
  };

  const createNewChat = async (title?: string): Promise<string | null> => {
    if (!authStore.userId) return null;
    try {
      closeSession();
      const newThread = await createAssistantChat(authStore.userId, title);
      chats.value.push(newThread);
      currentChatId.value = newThread.chat_id;
      messages.value = [];
      activeTools.value = [];
      streamingText.value = "";
      return newThread.chat_id;
    } catch (err) {
      console.error("Failed to create new chat:", err);
      return null;
    }
  };

  const deleteChat = async (chatId: string) => {
    if (!authStore.userId) return;
    try {
      await deleteAssistantChat(authStore.userId, chatId);
      chats.value = chats.value.filter((c) => c.chat_id !== chatId);
      if (currentChatId.value === chatId) {
        const nextChat = chats.value[0]?.chat_id || "default";
        await switchChat(nextChat);
      }
    } catch (err) {
      console.error("Failed to delete chat:", err);
    }
  };

  const clearMessages = async () => {
    if (!authStore.userId) return;
    try {
      closeSession();
      await clearAssistantHistory(authStore.userId, currentChatId.value);
      messages.value = [];
    } catch (err) {
      console.error("Failed to clear messages:", err);
    }
  };

  const cartMutatingTools = new Set([
    "add_to_cart",
    "update_cart_quantity",
    "remove_from_cart",
    "clear_cart",
    "apply_saved_contact_info",
    "set_shipping_address",
    "set_billing_address",
    "set_email",
    "checkout_cart",
  ]);

  const refreshCartIfCartToolsUsed = (toolNames: string[]) => {
    if (!authStore.userId) return;
    const touchedCart = toolNames.some((tool) => cartMutatingTools.has(tool));
    if (touchedCart) {
      cartStore.fetchCart(authStore.userId);
    }
  };

  // State for active in-flight turn
  let turnResolver: (() => void) | null = null;
  let turnRejecter: ((err: any) => void) | null = null;
  let turnUsedToolNames: string[] = [];
  let turnAccumulatedToolCalls: ToolCallRecord[] = [];
  let turnHadError = false;

  const handleChunk = (chunk: ChatChunk) => {
    switch (chunk._tag) {
      case "tool_start": {
        const tc = chunk.tool_call;
        const finalId = tc.id || `${tc.tool_name}-${Date.now()}`;
        turnUsedToolNames.push(tc.tool_name);
        activeTools.value.push({
          id: finalId,
          name: tc.tool_name,
          status: "running",
          summary: chunk.content,
        });
        turnAccumulatedToolCalls.push({
          id: finalId,
          tool_name: tc.tool_name,
          arguments: tc.arguments ?? "",
          result: "",
        });
        break;
      }
      case "tool_result": {
        const tc = chunk.tool_call;
        const finalId = tc.id;
        const tool = activeTools.value.find(
          (t) => (finalId ? t.id === finalId : t.name === tc.tool_name && t.status === "running"),
        );
        if (tool) {
          tool.status = "completed";
          tool.summary = chunk.content;
        }
        const existingTc = turnAccumulatedToolCalls.find(
          (t) => (finalId ? t.id === finalId : t.tool_name === tc.tool_name && !t.result),
        );
        if (existingTc) {
          existingTc.result = tc.result || chunk.content;
          if (tc.arguments && !existingTc.arguments) {
            existingTc.arguments = tc.arguments;
          }
        } else {
          turnAccumulatedToolCalls.push({
            id: finalId || `${tc.tool_name}-${Date.now()}`,
            tool_name: tc.tool_name,
            arguments: tc.arguments ?? "",
            result: tc.result || chunk.content,
          });
        }
        break;
      }
      case "text_delta":
        streamingText.value += chunk.content;
        break;
      case "error":
        turnHadError = true;
        error.value = new Error(chunk.content);
        break;
      case "done": {
        if (streamingText.value.trim()) {
          const assistantMessage: ChatMessage = {
            role: "assistant",
            content: streamingText.value,
            tool_calls: turnAccumulatedToolCalls.length > 0 ? [...turnAccumulatedToolCalls] : undefined,
            timestamp: new Date().toISOString(),
            type: turnHadError ? "error" : "ok",
          };
          messages.value.push(assistantMessage);
        } else {
          fetchHistory();
        }
        refreshCartIfCartToolsUsed(turnUsedToolNames);
        if (turnResolver) {
          const res = turnResolver;
          turnResolver = null;
          turnRejecter = null;
          res();
        }
        break;
      }
    }
  };

  const ensureSession = async (): Promise<ChatSession> => {
    if (activeSession.value) return activeSession.value;
    const session = await createChatSession(
      authStore.userId!,
      (chunk) => handleChunk(chunk),
      (err) => {
        closeSession();
        if (turnRejecter) {
          const rej = turnRejecter;
          turnResolver = null;
          turnRejecter = null;
          rej(err);
        }
      },
      undefined,
      currentChatId.value,
    );
    activeSession.value = session;
    return session;
  };

  const sendMessage = async (content: string, stream = true) => {
    if (!authStore.userId || !content.trim()) return;

    const userMessage: ChatMessage = {
      role: "user",
      content,
      timestamp: new Date().toISOString(),
      type: "ok",
    };
    messages.value.push(userMessage);

    isLoading.value = true;
    error.value = null;
    activeTools.value = [];
    streamingText.value = "";
    turnUsedToolNames = [];
    turnAccumulatedToolCalls = [];
    turnHadError = false;

    if (stream) {
      isStreaming.value = true;
      try {
        // 1. Try bi-directional streaming session
        const session = await ensureSession();
        await new Promise<void>((resolve, reject) => {
          turnResolver = resolve;
          turnRejecter = reject;
          session.sendMessage(content).catch((sendErr) => {
            turnResolver = null;
            turnRejecter = null;
            reject(sendErr);
          });
        });
      } catch (sessionErr) {
        console.warn("Session stream failed, falling back to synchronous chat:", sessionErr);
        closeSession();
        try {
          const res = await chatWithAssistant(authStore.userId, content, currentChatId.value);
          messages.value.push({
            role: "assistant",
            content: res.reply,
            tool_calls: res.tool_calls_used,
            timestamp: new Date().toISOString(),
            type: res.type ?? "ok",
          });
          refreshCartIfCartToolsUsed(res.tool_calls_used.map((tc) => tc.tool_name));
        } catch (syncErr) {
          error.value = syncErr as Error;
        }
      } finally {
        isStreaming.value = false;
        isLoading.value = false;
        streamingText.value = "";
      }
    } else {
      try {
        const res = await chatWithAssistant(authStore.userId, content, currentChatId.value);
        messages.value.push({
          role: "assistant",
          content: res.reply,
          tool_calls: res.tool_calls_used,
          timestamp: new Date().toISOString(),
          type: res.type ?? "ok",
        });
        refreshCartIfCartToolsUsed(res.tool_calls_used.map((tc) => tc.tool_name));
      } catch (err) {
        error.value = err as Error;
      } finally {
        isLoading.value = false;
      }
    }
  };

  const fetchRecommendedProducts = async (options?: PriceFilterOptions) => {
    if (authStore.userId) {
      isLoading.value = true;
      error.value = null;

      try {
        recommendedProducts.value = await getRecommendedProducts(authStore.userId, options);
      } catch (err) {
        error.value = err as Error;
        console.error("Error getting recommended products:", err);
      } finally {
        isLoading.value = false;
      }
    } else {
      clearRecommendations();
    }
  };

  const clearRecommendations = () => {
    recommendedProducts.value = [];
    lastUpdated.value = null;
  };

  return {
    recommendedProducts,
    messages,
    chats,
    currentChatId,
    isLoading,
    isStreaming,
    activeTools,
    streamingText,
    error,
    lastUpdated,
    hasRecommendations,
    hasMessages,
    fetchChats,
    fetchHistory,
    switchChat,
    createNewChat,
    deleteChat,
    clearMessages,
    sendMessage,
    closeSession,
    fetchRecommendedProducts,
    clearRecommendations,
  };
});
