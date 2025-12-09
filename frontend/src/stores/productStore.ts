import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { PriceFilterOptions } from "@/api/services/pricingService";
import {
  searchProducts,
  getProductById,
  type Product,
} from "@/api/services/productService";

export const useProductStore = defineStore("products", () => {
  const products = ref<Product[]>([]);
  const currentProduct = ref<Product | null>(null);
  const isLoading = ref(false);
  const error = ref<Error | null>(null);

  const search = async (query: string, options?: PriceFilterOptions) => {
    isLoading.value = true;
    error.value = null;

    try {
      const results = await searchProducts(query, options);
      products.value = results;
    } catch (err) {
      error.value = err as Error;
      console.error("Error searching products:", err);
    } finally {
      isLoading.value = false;
    }
  };

  const fetchProduct = async (
    productId: string,
    options?: PriceFilterOptions,
  ) => {
    isLoading.value = true;
    error.value = null;

    try {
      const product = await getProductById(productId, true, options);
      currentProduct.value = product;
      return product;
    } catch (err) {
      error.value = err as Error;
      console.error(`Error fetching product ${productId}:`, err);
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const clearProducts = () => {
    products.value = [];
    currentProduct.value = null;
  };

  const hasProducts = computed(() => products.value.length > 0);

  return {
    products,
    currentProduct,
    isLoading,
    error,
    hasProducts,
    search,
    fetchProduct,
    clearProducts,
  };
});
