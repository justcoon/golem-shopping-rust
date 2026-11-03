<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useProductStore } from "@/stores/productStore";
import { useCartStore } from "@/stores/cartStore";
import { useAuthStore } from "@/stores/authStore";
import { useAssistantStore } from "@/stores/assistantStore.ts";
import ProductCard from "@/components/ProductCard.vue";
import type { Product } from "@/api/services/productService.ts";
import { useRouter } from "vue-router";

const router = useRouter();
const productStore = useProductStore();
const cartStore = useCartStore();
const authStore = useAuthStore();
const assistantStore = useAssistantStore();

const isAddingToCart = ref(false);
const currentUserId = authStore.userId;

// Fetch products when component mounts
onMounted(async () => {
  if (productStore.products.length === 0) {
    await productStore.search("", authStore.pricePreferences);
  }
  if (assistantStore.recommendedProducts.length === 0) {
    await assistantStore.fetchRecommendedProducts(authStore.pricePreferences);
  }
});

const featuredProducts = computed(() => {
  return productStore.products.slice(0, 4);
});

const recommendedProducts = computed(() => {
  return assistantStore.recommendedProducts;
});

const isProductStoreLoading = computed(() => productStore.isLoading);
const productStoreError = computed(() => productStore.error);
const isAssistantStoreLoading = computed(() => assistantStore.isLoading);
const assistantStoreError = computed(() => assistantStore.error);

const addToCart = async (product: Product) => {
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
};
</script>

<template>
  <div class="home-view">
    <!-- Hero Section with Glowing Glass Card -->
    <section class="hero-section container">
      <div class="hero-card glass">
        <div class="hero-badge">
          <span class="badge-dot"></span>
          <span>Powered by Golem Durable Agents</span>
        </div>
        <h1 class="hero-title">
          The Future of <span class="gradient-text">Agentic Commerce</span>
        </h1>
        <p class="hero-lead">
          Intelligent product discovery, real-time pricing calculation, and 1-click checkout powered
          by autonomous AI agents.
        </p>
        <div class="hero-actions">
          <router-link to="/products" class="btn btn-primary btn-lg">
            <span>Explore Catalog</span>
            <span>→</span>
          </router-link>
          <router-link
            v-if="authStore.isAuthenticated"
            to="/assistant"
            class="btn btn-outline btn-lg"
          >
            <span>🤖 AI Shopping Assistant</span>
          </router-link>
        </div>
      </div>
    </section>

    <!-- Featured Products -->
    <section class="products-section container">
      <div class="section-header">
        <div>
          <h2 class="gradient-text">Featured Products</h2>
          <p class="section-subtitle">Top curated items across our inventory</p>
        </div>
        <router-link to="/products" class="view-all-link"> View all → </router-link>
      </div>

      <div v-if="isProductStoreLoading" class="loading-state glass">
        <span class="spinner"></span>
        <span>Loading catalog items...</span>
      </div>
      <div v-else-if="productStoreError" class="error-state glass">
        ⚠️ Error loading products: {{ productStoreError.message }}
      </div>
      <div v-else class="product-grid">
        <ProductCard
          v-for="product in featuredProducts"
          :key="product.product_id"
          :product="product"
          :is-adding-to-cart="isAddingToCart"
          @add-to-cart="addToCart"
        />
      </div>
    </section>

    <!-- Recommended Products (if available) -->
    <section v-if="recommendedProducts.length > 0" class="products-section container">
      <div class="section-header">
        <div>
          <h2 class="gradient-accent">Personalized For You</h2>
          <p class="section-subtitle">Tailored recommendations based on your purchase history</p>
        </div>
      </div>

      <div v-if="isAssistantStoreLoading" class="loading-state glass">
        <span class="spinner"></span>
        <span>Generating AI recommendations...</span>
      </div>
      <div v-else-if="assistantStoreError" class="error-state glass">
        ⚠️ {{ assistantStoreError.message }}
      </div>
      <div v-else class="product-grid">
        <ProductCard
          v-for="product in recommendedProducts"
          :key="product.product_id"
          :product="product"
          :is-adding-to-cart="isAddingToCart"
          @add-to-cart="addToCart"
        />
      </div>
    </section>
  </div>
</template>

<style scoped>
.home-view {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 40px;
}

.hero-section {
  width: 100%;
  padding-top: 10px;
}

.hero-card {
  width: 100%;
  padding: 60px 30px;
  text-align: center;
  background: radial-gradient(
    circle at 50% 0%,
    rgba(139, 92, 246, 0.2) 0%,
    rgba(17, 24, 39, 0.75) 100%
  );
  border: 1px solid rgba(139, 92, 246, 0.25);
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
}

.hero-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  background: rgba(139, 92, 246, 0.15);
  border: 1px solid rgba(139, 92, 246, 0.3);
  padding: 6px 14px;
  border-radius: 20px;
  font-size: 0.8rem;
  font-weight: 600;
  color: #c084fc;
  margin-bottom: 20px;
}

.badge-dot {
  width: 8px;
  height: 8px;
  background: #10b981;
  border-radius: 50%;
  box-shadow: 0 0 8px #10b981;
}

.hero-title {
  font-size: 3rem;
  font-weight: 800;
  line-height: 1.15;
  margin-bottom: 16px;
}

.hero-lead {
  font-size: 1.15rem;
  color: var(--text-muted);
  max-width: 650px;
  margin: 0 auto 30px;
  line-height: 1.6;
}

.hero-actions {
  display: flex;
  gap: 16px;
  justify-content: center;
  flex-wrap: wrap;
}

.products-section {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
}

.section-subtitle {
  font-size: 0.9rem;
  color: var(--text-muted);
  margin-top: 4px;
}

.view-all-link {
  font-size: 0.9rem;
  font-weight: 600;
  color: #c084fc;
  display: flex;
  align-items: center;
  gap: 4px;
}

.view-all-link:hover {
  color: #e9d5ff;
}

.product-grid {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 22px;
}

.loading-state,
.error-state {
  padding: 30px;
  text-align: center;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-muted);
}

.error-state {
  color: #f87171;
  border-color: rgba(239, 68, 68, 0.3);
}

@media (max-width: 1100px) {
  .product-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}

@media (max-width: 820px) {
  .product-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 768px) {
  .hero-title {
    font-size: 2.2rem;
  }
}

@media (max-width: 560px) {
  .product-grid {
    grid-template-columns: 1fr;
  }
}
</style>
