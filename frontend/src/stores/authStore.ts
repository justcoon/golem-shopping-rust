import { defineStore } from "pinia";
import { ref } from "vue";
import {useAssistantStore} from "@/stores/assistantStore.ts";
import {useCartStore} from "@/stores/cartStore.ts";

export interface PricePreferences {
  currency: string;
  zone: string;
}

const DEFAULT_PREFERENCES: PricePreferences = {
  currency: "USD",
  zone: "global",
};

export const useAuthStore = defineStore("auth", () => {
  const userId = ref<string | null>(localStorage.getItem("userId"));
  const isAuthenticated = ref(!!userId.value);

  const assistantStore = useAssistantStore();
  const cartStore = useCartStore();

  // Default price preferences
  const pricePreferences = ref<PricePreferences>({ ...DEFAULT_PREFERENCES });

  const login = (id: string) => {
    userId.value = id;
    isAuthenticated.value = true;
    localStorage.setItem("userId", id);
  };

  const logout = () => {
    userId.value = null;
    isAuthenticated.value = false;
    localStorage.removeItem("userId");
    assistantStore.clearRecommendations();
    cartStore.clearCart();
  };

  return {
    // Auth state
    userId,
    isAuthenticated,
    login,
    logout,

    // Price preferences
    pricePreferences,
  };
});
