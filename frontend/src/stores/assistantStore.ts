import {defineStore} from "pinia";
import {ref, computed} from "vue";
import {getRecommendedProducts} from "@/api/services/assistantService";
import {useAuthStore} from "./authStore";
import type {PriceFilterOptions} from "@/api/services/pricingService.ts";
import type {Product} from "@/api/services/productService.ts";


export const useAssistantStore = defineStore("assistant", () => {
    const recommendedProducts = ref<Product[]>([]);
    const isLoading = ref(false);
    const error = ref<Error | null>(null);
    const lastUpdated = ref<Date | null>(null);

    const authStore = useAuthStore();

    const hasRecommendations = computed(() => recommendedProducts.value.length > 0);

    const fetchRecommendedProducts = async (options?: PriceFilterOptions) => {
        if (authStore.userId) {
            isLoading.value = true;
            error.value = null;

            try {
                const results = await getRecommendedProducts(authStore.userId, options);
                recommendedProducts.value = results;
            } catch (err) {
                error.value = err as Error;
                console.error("Error getting recommended products:", err);
            } finally {
                isLoading.value = false;
            }
        } else {
            clearRecommendations()
        }
    };

    const clearRecommendations = () => {
        recommendedProducts.value = [];
        lastUpdated.value = null;
    };

    return {
        // State
        recommendedProducts,
        isLoading,
        error,
        lastUpdated,

        // Getters
        hasRecommendations,

        // Actions
        fetchRecommendedProducts,
        clearRecommendations,
    };
});
