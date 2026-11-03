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

    if (!availableBrands.value.includes(brand)) {
      try {
        await productStore.search("");
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
const totalPages = computed(() => Math.ceil(filteredProducts.value.length / itemsPerPage));
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
  await productStore.search(searchQuery.value || "", authStore.pricePreferences);
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
    await cartStore.addItem(currentUserId, product.product_id, 1);
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
  currentPage.value = 1;
});

// Watch for changes in route query
watch(
  () => route.query,
  async (newQuery) => {
    if (newQuery.brand !== selectedBrand.value) {
      selectedBrand.value = (newQuery.brand as string) || "";

      if (newQuery.brand && !availableBrands.value.includes(newQuery.brand as string)) {
        try {
          await productStore.search("", authStore.pricePreferences);
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
  <div class="product-list-view container">
    <div class="page-header">
      <h1 class="gradient-text">Product Catalog</h1>
      <p class="subtitle">Discover our collection with real-time dynamic pricing</p>
    </div>

    <!-- Search & Filters Toolbar -->
    <div class="toolbar glass">
      <div class="search-box">
        <span class="search-icon">🔍</span>
        <input
          v-model="searchQuery"
          placeholder="Search products by title, tag, or brand..."
          class="search-input"
          @input="debouncedSearch"
        />
        <button
          v-if="searchQuery"
          class="clear-btn"
          @click="
            searchQuery = '';
            debouncedSearch();
          "
        >
          ✕
        </button>
      </div>

      <div class="filter-controls">
        <select v-model="selectedBrand" class="custom-select">
          <option value="">All Brands</option>
          <option v-for="brand in availableBrands" :key="brand" :value="brand">
            {{ brand }}
          </option>
        </select>

        <select v-model="sortBy" class="custom-select">
          <option value="name-asc">Name (A-Z)</option>
          <option value="name-desc">Name (Z-A)</option>
          <option value="price-asc">Price (Low to High)</option>
          <option value="price-desc">Price (High to Low)</option>
        </select>
      </div>
    </div>

    <div v-if="isLoading" class="state-card glass">
      <span class="spinner"></span>
      <span>Loading catalog items...</span>
    </div>

    <div v-else-if="error" class="state-card glass error-card">
      <span>⚠️ Error: {{ error.message }}</span>
      <button class="btn btn-outline btn-sm" @click="retryLoading">Retry</button>
    </div>

    <div v-else>
      <div v-if="filteredProducts.length === 0" class="state-card glass">
        <p>No products match your criteria.</p>
        <button class="btn btn-primary btn-sm" @click="clearFilters">Clear Filters</button>
      </div>

      <div v-else class="product-grid">
        <ProductCard
          v-for="product in paginatedProducts"
          :key="product.product_id"
          :product="product"
          :is-adding-to-cart="isAddingToCart"
          @add-to-cart="addToCart"
        />
      </div>

      <div v-if="totalPages > 1" class="pagination glass">
        <button
          class="btn btn-secondary btn-sm"
          :disabled="currentPage === 1"
          @click="currentPage--"
        >
          ← Previous
        </button>
        <span class="page-info">
          Page <strong>{{ currentPage }}</strong> of <strong>{{ totalPages }}</strong>
        </span>
        <button
          class="btn btn-secondary btn-sm"
          :disabled="currentPage >= totalPages"
          @click="currentPage++"
        >
          Next →
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.product-list-view {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.page-header {
  margin-bottom: 4px;
}

.subtitle {
  color: var(--text-muted);
  font-size: 0.95rem;
  margin-top: 4px;
}

.toolbar {
  padding: 16px 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
}

.search-box {
  position: relative;
  flex: 1;
  min-width: 260px;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: 14px;
  font-size: 0.9rem;
  opacity: 0.6;
}

.search-input {
  width: 100%;
  padding-left: 38px;
  padding-right: 36px;
}

.clear-btn {
  position: absolute;
  right: 10px;
  background: transparent;
  border: none;
  color: var(--text-dim);
  font-size: 0.8rem;
  cursor: pointer;
  padding: 4px;
}

.clear-btn:hover {
  color: #ef4444;
}

.filter-controls {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.custom-select {
  cursor: pointer;
}

.product-grid {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 22px;
}

.state-card {
  padding: 40px 20px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  color: var(--text-muted);
}

.error-card {
  color: #f87171;
  border-color: rgba(239, 68, 68, 0.3);
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 16px;
  padding: 14px 20px;
  margin-top: 12px;
}

.page-info {
  font-size: 0.9rem;
  color: var(--text-muted);
}

.page-info strong {
  color: #38bdf8;
  font-family: var(--font-mono);
}
</style>
