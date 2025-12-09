<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useProductStore } from "@/stores/productStore";
import { useCartStore } from "@/stores/cartStore";
import { useAuthStore } from "@/stores/authStore";
import { debounce } from "lodash-es";
import ProductCard from "@/components/ProductCard.vue";
import type { Product } from "@/api/services/productService.ts";

const router = useRouter();
const route = useRoute();
const productStore = useProductStore();
const cartStore = useCartStore();

// State
const searchQuery = ref("");
const sortBy = ref("name-asc");
const selectedBrand = ref("");

// Initialize from URL query parameters
const initializeFromQuery = async () => {
  const { brand } = route.query;
  if (brand && typeof brand === "string") {
    selectedBrand.value = brand;

    // If the brand isn't in available brands, trigger a search
    if (!availableBrands.value.includes(brand)) {
      try {
        await productStore.search("");
        // If still not found after search, clear the filter
        if (!availableBrands.value.includes(brand)) {
          selectedBrand.value = "";
        }
      } catch (error) {
        console.error("Error searching for products:", error);
        selectedBrand.value = "";
      }
    }
  } else {
    selectedBrand.value = "";
  }
};
const currentPage = ref(1);
const itemsPerPage = 12;
const isAddingToCart = ref(false);
const authStore = useAuthStore();
const currentUserId = authStore.userId;

const { isLoading, error } = productStore;
const allProducts = computed(() => productStore.products);

const availableBrands = computed(() => {
  const brands = new Set<string>();
  allProducts.value.forEach((p) => p.brand && brands.add(p.brand));
  return Array.from(brands).sort();
});

const filteredProducts = computed(() => {
  let result = [...allProducts.value];

  // Brand filter
  if (selectedBrand.value) {
    result = result.filter((product) => product.brand === selectedBrand.value);
  }

  // Search filter
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase();
    result = result.filter(
      (p) =>
        p.name.toLowerCase().includes(query) ||
        p.brand.toLowerCase().includes(query) ||
        (p.tags && p.tags.some((t: string) => t.toLowerCase().includes(query))),
    );
  }

  // Sorting
  return result.sort((a, b) => {
    switch (sortBy.value) {
      case "name-asc":
        return a.name.localeCompare(b.name);
      case "name-desc":
        return b.name.localeCompare(a.name);
      case "price-asc":
        return (a.bestPrice || 0) - (b.bestPrice || 0);
      case "price-desc":
        return (b.bestPrice || 0) - (a.bestPrice || 0);
      default:
        return 0;
    }
  });
});

// Pagination
const totalPages = computed(() =>
  Math.ceil(filteredProducts.value.length / itemsPerPage),
);
const paginatedProducts = computed(() => {
  const start = (currentPage.value - 1) * itemsPerPage;
  return filteredProducts.value.slice(start, start + itemsPerPage);
});

// Watchers
watch([searchQuery, selectedBrand, sortBy], () => {
  currentPage.value = 1;
});

// Methods
const debouncedSearch = debounce(performSearch, 300);

async function performSearch() {
  await productStore.search(
    searchQuery.value || "",
    authStore.pricePreferences,
  );
}

async function retryLoading() {
  await performSearch();
}

function clearFilters() {
  searchQuery.value = "";
  selectedBrand.value = "";
  sortBy.value = "name-asc";
  currentPage.value = 1;
}

async function addToCart(product: Product) {
  if (!authStore.isAuthenticated) {
    router.push({
      name: "login",
      query: { redirect: router.currentRoute.value.fullPath },
    });
    return;
  }

  try {
    isAddingToCart.value = true;
    await cartStore.addItem(currentUserId, product["product-id"], 1);
  } catch (err) {
    console.error("Error adding to cart:", err);
  } finally {
    isAddingToCart.value = false;
  }
}

// Watch for changes in the selected brand and update URL
watch(selectedBrand, (newBrand) => {
  router.push({
    query: {
      ...route.query,
      brand: newBrand || undefined,
    },
  });
  currentPage.value = 1; // Reset to first page when brand changes
});

// Watch for changes in route query
watch(
  () => route.query,
  async (newQuery) => {
    if (newQuery.brand !== selectedBrand.value) {
      selectedBrand.value = newQuery.brand || "";

      // If we have a brand in the URL but it's not in available brands, trigger a search
      if (
        newQuery.brand &&
        !availableBrands.value.includes(newQuery.brand as string)
      ) {
        try {
          await productStore.search("", authStore.pricePreferences);
          // If the brand is still not found after search, clear the filter
          if (!availableBrands.value.includes(newQuery.brand as string)) {
            selectedBrand.value = "";
          }
        } catch (error) {
          console.error("Error searching for products:", error);
          selectedBrand.value = "";
        }
      }
    }
  },
  { immediate: true },
);

// Lifecycle
onMounted(async () => {
  initializeFromQuery();
  if (allProducts.value.length === 0) {
    performSearch();
  }
});
</script>

<template>
  <div class="product-list-view">
    <div class="container">
      <h1>Products</h1>
      <div class="search-container">
        <input
          v-model="searchQuery"
          placeholder="Search products..."
          class="search-input"
          @input="debouncedSearch"
        />
      </div>

      <div v-if="isLoading" class="loading">Loading products...</div>

      <div v-else-if="error" class="error">
        Error: {{ error.message }}
        <button class="btn btn-outline-primary" @click="retryLoading">
          Retry
        </button>
      </div>

      <div v-else>
        <div class="filters">
          <select v-model="sortBy" class="form-select">
            <option value="name-asc">Name (A-Z)</option>
            <option value="name-desc">Name (Z-A)</option>
            <option value="price-asc">Price (Low to High)</option>
            <option value="price-desc">Price (High to Low)</option>
          </select>

          <select v-model="selectedBrand" class="form-select">
            <option value="">All Brands</option>
            <option
              v-for="brand in availableBrands"
              :key="brand"
              :value="brand"
            >
              {{ brand }}
            </option>
          </select>
        </div>

        <div v-if="filteredProducts.length === 0" class="no-results">
          No products found.
          <button class="btn btn-primary" @click="clearFilters">
            Clear Filters
          </button>
        </div>

        <div v-else class="product-grid">
          <ProductCard
            v-for="product in paginatedProducts"
            :key="product['product-id']"
            :product="product"
            :is-adding-to-cart="isAddingToCart"
            @add-to-cart="addToCart"
          />
        </div>

        <div v-if="totalPages > 1" class="pagination">
          <button :disabled="currentPage === 1" @click="currentPage--">
            Previous
          </button>
          <span>Page {{ currentPage }} of {{ totalPages }}</span>
          <button :disabled="currentPage >= totalPages" @click="currentPage++">
            Next
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.product-list-view {
  padding: 2rem 0;
}

.product-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 2rem;
  margin: 2rem 0;
}

.loading,
.error,
.no-results {
  text-align: center;
  padding: 2rem;
  font-size: 1.1rem;
}

.error {
  color: #dc3545;
}

.no-results {
  color: #666;
}

.filters {
  display: flex;
  gap: 1rem;
  margin: 1rem 0;
}

.search-container {
  margin: 1rem 0;
  display: flex;
  gap: 0.5rem;
}

.search-input {
  flex: 1;
  padding: 0.5rem;
  border: 1px solid #ddd;
  border-radius: 4px;
}

.form-select {
  padding: 0.5rem;
  border: 1px solid #ddd;
  border-radius: 4px;
  background-color: white;
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  margin: 2rem 0;
}

.pagination button {
  padding: 0.5rem 1rem;
  border: 1px solid #ddd;
  background: white;
  cursor: pointer;
  border-radius: 4px;
}

.pagination button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.2s;
}

.btn-primary {
  background-color: #4a6fa5;
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background-color: #3a5a80;
}

.btn-outline-primary {
  background: none;
  border: 1px solid #4a6fa5;
  color: #4a6fa5;
}

.btn-outline-primary:hover {
  background-color: #f0f4f8;
}

.search-container {
  margin: 1rem 0;
  display: flex;
  gap: 0.5rem;
}

.search-input {
  flex: 1;
  padding: 0.5rem;
  border: 1px solid #ddd;
  border-radius: 4px;
}

.filters {
  display: flex;
  gap: 1rem;
  margin: 1rem 0;
}

.form-select {
  padding: 0.5rem;
  border: 1px solid #ddd;
  border-radius: 4px;
  background-color: white;
}

.product-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 2rem;
  margin: 2rem 0;
}

.product-card {
  border: 1px solid #eee;
  border-radius: 8px;
  overflow: hidden;
  transition: transform 0.2s;
}

.product-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.product-image {
  position: relative;
  height: 200px;
  background: #f8f9fa;
}

.product-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.sale-badge {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  background: #dc3545;
  color: white;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: 0.8rem;
  font-weight: bold;
}

.product-details {
  padding: 1rem;
}

.product-details h3 {
  margin: 0 0 0.5rem 0;
  font-size: 1.1rem;
}

.product-details h3 a {
  color: #333;
  text-decoration: none;
}

.product-details h3 a:hover {
  color: #4a6fa5;
}

.brand {
  color: #666;
  margin: 0 0 0.5rem 0;
  font-size: 0.9rem;
}

.price {
  font-weight: bold;
  font-size: 1.2rem;
  margin: 0.5rem 0;
  color: #4a6fa5;
}

.original-price {
  text-decoration: line-through;
  color: #999;
  font-size: 0.9rem;
  margin-left: 0.5rem;
}

.product-details button {
  width: 100%;
  margin-top: 0.5rem;
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  margin: 2rem 0;
}

.pagination button {
  padding: 0.5rem 1rem;
  border: 1px solid #ddd;
  background: white;
  cursor: pointer;
  border-radius: 4px;
}

.pagination button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.loading,
.error,
.no-results {
  text-align: center;
  padding: 2rem;
  font-size: 1.1rem;
}

.error {
  color: #dc3545;
}

.no-results {
  color: #666;
}
</style>
