<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useProductStore } from "@/stores/productStore";
import { useCartStore } from "@/stores/cartStore";
import { formatPrice } from "@/utils/currency";
import { getProductImage } from "@/api/services/productService";
import { useAuthStore } from "@/stores/authStore";
import {
  getProductBestPrice,
  isProductOnSale,
  getProductOriginalPrice,
} from "@/api/services/productService";

const router = useRouter();
const route = useRoute();
const productStore = useProductStore();
const cartStore = useCartStore();

const productId = computed(() => route.params.id as string);
const product = computed(() => productStore.currentProduct);
const isLoading = computed(() => productStore.isLoading);
const error = computed(() => productStore.error);
const isAddingToCart = ref(false);
const qty = ref(1);
const mainImage = ref("");
const authStore = useAuthStore();
const currentUserId = authStore.userId;

// Computed
const productImages = computed(() => {
  const baseImg = getProductImage({ name: product.value?.name || "Product" });
  return [baseImg];
});

const bestPrice = computed(() => {
  if (!product.value) return "0.00";
  return getProductBestPrice(product.value, authStore.pricePreferences);
});

const originalPrice = computed(() => {
  if (!product.value) return "0.00";
  return getProductOriginalPrice(product.value, authStore.pricePreferences);
});

const hasDiscount = computed(() => {
  return product.value && isProductOnSale(product.value, authStore.pricePreferences);
});

const discountPercentage = computed(() => {
  if (!hasDiscount.value) return 0;
  const bestSalePrice = parseFloat(bestPrice.value);
  const listPrice = parseFloat(originalPrice.value);
  const discount = 100 - (bestSalePrice / listPrice) * 100;
  return Math.round(discount);
});

function increaseQty() {
  qty.value++;
}
function decreaseQty() {
  if (qty.value > 1) qty.value--;
}

async function addToCart() {
  if (!product.value) return;

  if (!authStore.isAuthenticated) {
    router.push({ name: "login", query: { redirect: route.fullPath } });
    return;
  }

  try {
    isAddingToCart.value = true;
    await cartStore.addItem(currentUserId, product.value.product_id, qty.value);
  } catch (err) {
    console.error("Error adding to cart:", err);
  } finally {
    isAddingToCart.value = false;
  }
}

async function fetchProduct() {
  if (productId.value) {
    await productStore.fetchProduct(productId.value, authStore.pricePreferences);
    if (product.value) {
      mainImage.value = getProductImage({ name: product.value.name });
    }
  }
}

// Lifecycle
onMounted(fetchProduct);
watch(() => route.params.id, fetchProduct);
</script>

<template>
  <div class="product-detail-view">
    <div class="back-nav">
      <router-link to="/products" class="btn-back"> <span>←</span> Back to Catalog </router-link>
    </div>

    <!-- Loading State -->
    <div v-if="isLoading" class="state-card glass-card">
      <div class="spinner"></div>
      <p>Loading product details...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="state-card glass-card error-card">
      <div class="error-icon">⚠️</div>
      <h3>Error Loading Product</h3>
      <p>{{ error.message }}</p>
      <button class="btn btn-secondary" @click="fetchProduct">Retry</button>
    </div>

    <!-- Product Container -->
    <div v-else-if="product" class="product-container glass-card">
      <!-- Gallery Column -->
      <div class="gallery-col">
        <div class="main-image-wrap">
          <img
            :src="mainImage || getProductImage(product)"
            :alt="product.name"
            class="main-image"
          />
          <span v-if="hasDiscount" class="sale-pill"> 🔥 -{{ discountPercentage }}% OFF </span>
        </div>
        <div class="thumbnails" v-if="productImages.length > 1">
          <img
            v-for="(img, i) in productImages"
            :key="i"
            :src="img"
            :class="{ active: mainImage === img }"
            @click="mainImage = img"
          />
        </div>
      </div>

      <!-- Info Column -->
      <div class="info-col">
        <div class="product-header">
          <div class="brand-row">
            <router-link
              v-if="product.brand"
              :to="{ name: 'products', query: { brand: product.brand } }"
              class="brand-pill"
            >
              🏷️ {{ product.brand }}
            </router-link>
            <span class="sku-pill">ID: {{ product.product_id }}</span>
          </div>
          <h1 class="product-title">{{ product.name }}</h1>
        </div>

        <!-- Pricing Section -->
        <div class="price-section">
          <div class="price-box">
            <span class="current-price">
              {{ formatPrice(bestPrice, authStore.pricePreferences.currency) }}
            </span>
            <span v-if="hasDiscount" class="original-price">
              {{ formatPrice(originalPrice, authStore.pricePreferences.currency) }}
            </span>
          </div>
          <div class="price-meta">Live dynamic rate evaluated via autonomous pricing agents</div>
        </div>

        <!-- Description -->
        <div class="description-section">
          <h3>Overview</h3>
          <p>{{ product.description || "No description provided for this catalog item." }}</p>
        </div>

        <!-- Tags -->
        <div v-if="product.tags?.length" class="tags-section">
          <span v-for="tag in product.tags" :key="tag" class="tag-badge"> #{{ tag }} </span>
        </div>

        <!-- Add to Cart Actions -->
        <div class="actions-section">
          <div class="quantity-picker">
            <button class="qty-btn" :disabled="qty <= 1" @click="decreaseQty">−</button>
            <input v-model.number="qty" type="number" min="1" class="qty-input" />
            <button class="qty-btn" @click="increaseQty">+</button>
          </div>

          <button
            class="btn btn-primary add-cart-btn"
            :disabled="isAddingToCart"
            @click="addToCart"
          >
            <span v-if="isAddingToCart" class="spinner"></span>
            <span v-else>🛍️ Add to Cart</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Not Found State -->
    <div v-else class="state-card glass-card">
      <div class="error-icon">🔍</div>
      <h2>Product Not Found</h2>
      <p>The product you are looking for might have been moved or discontinued.</p>
      <router-link to="/products" class="btn btn-primary"> Browse Catalog </router-link>
    </div>
  </div>
</template>

<style scoped>
.product-detail-view {
  max-width: 1200px;
  margin: 0 auto;
  padding: 1.5rem 1rem 3rem;
}

.back-nav {
  margin-bottom: 1.5rem;
}

.btn-back {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--text-muted);
  text-decoration: none;
  font-size: 0.9rem;
  padding: 0.4rem 0.8rem;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-color);
  transition: all 0.2s;
}

.btn-back:hover {
  color: var(--text-main);
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.2);
}

.state-card {
  text-align: center;
  padding: 4rem 2rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
}

.error-icon {
  font-size: 3rem;
}

.error-card {
  border-color: rgba(239, 68, 68, 0.3);
}

.product-container {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 3rem;
  padding: 2.5rem;
}

/* Gallery Column */
.gallery-col {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.main-image-wrap {
  position: relative;
  width: 100%;
  aspect-ratio: 4 / 3;
  border-radius: 12px;
  overflow: hidden;
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: center;
}

.main-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: transform 0.4s ease;
}

.main-image-wrap:hover .main-image {
  transform: scale(1.03);
}

.sale-pill {
  position: absolute;
  top: 1rem;
  left: 1rem;
  background: linear-gradient(135deg, #ec4899 0%, #f43f5e 100%);
  color: white;
  font-size: 0.8rem;
  font-weight: 700;
  padding: 0.35rem 0.8rem;
  border-radius: 9999px;
  box-shadow: 0 4px 12px rgba(236, 72, 153, 0.4);
}

.thumbnails {
  display: flex;
  gap: 0.75rem;
}

.thumbnails img {
  width: 70px;
  height: 70px;
  object-fit: cover;
  border-radius: 8px;
  cursor: pointer;
  border: 1px solid var(--border-color);
  transition: all 0.2s;
}

.thumbnails img.active,
.thumbnails img:hover {
  border-color: var(--primary);
  box-shadow: 0 0 10px var(--primary-glow);
}

/* Info Column */
.info-col {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.brand-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}

.brand-pill {
  display: inline-flex;
  align-items: center;
  font-size: 0.8rem;
  font-weight: 600;
  padding: 0.25rem 0.75rem;
  border-radius: 9999px;
  background: rgba(139, 92, 246, 0.15);
  color: #c084fc;
  border: 1px solid rgba(139, 92, 246, 0.3);
  text-decoration: none;
  transition: all 0.2s;
}

.brand-pill:hover {
  background: rgba(139, 92, 246, 0.25);
  border-color: #c084fc;
}

.sku-pill {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.75rem;
  color: var(--text-muted);
}

.product-title {
  font-size: 2rem;
  font-weight: 700;
  line-height: 1.25;
  color: var(--text-main);
  margin: 0;
}

/* Pricing */
.price-section {
  padding: 1.25rem;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border-color);
}

.price-box {
  display: flex;
  align-items: baseline;
  gap: 1rem;
}

.current-price {
  font-family: "JetBrains Mono", monospace;
  font-size: 2.2rem;
  font-weight: 700;
  color: var(--accent-cyan);
  text-shadow: 0 0 20px rgba(6, 182, 212, 0.3);
}

.original-price {
  font-family: "JetBrains Mono", monospace;
  font-size: 1.3rem;
  text-decoration: line-through;
  color: var(--text-muted);
}

.price-meta {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.4rem;
}

/* Description */
.description-section h3 {
  font-size: 1rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 0.5rem;
}

.description-section p {
  color: var(--text-main);
  line-height: 1.6;
  font-size: 0.95rem;
  opacity: 0.9;
}

/* Tags */
.tags-section {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.tag-badge {
  font-size: 0.75rem;
  padding: 0.25rem 0.6rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-muted);
  border: 1px solid var(--border-color);
}

/* Actions */
.actions-section {
  display: flex;
  align-items: center;
  gap: 1.25rem;
  margin-top: 0.5rem;
}

.quantity-picker {
  display: flex;
  align-items: center;
  background: rgba(15, 23, 42, 0.8);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  overflow: hidden;
}

.qty-btn {
  width: 44px;
  height: 44px;
  background: transparent;
  border: none;
  color: var(--text-main);
  font-size: 1.25rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s;
}

.qty-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.1);
  color: var(--primary);
}

.qty-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.qty-input {
  width: 50px;
  height: 44px;
  background: transparent;
  border: none;
  text-align: center;
  color: var(--text-main);
  font-family: "JetBrains Mono", monospace;
  font-size: 1rem;
  font-weight: 600;
  border-left: 1px solid var(--border-color);
  border-right: 1px solid var(--border-color);
  -moz-appearance: textfield;
}

.qty-input::-webkit-outer-spin-button,
.qty-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.add-cart-btn {
  flex: 1;
  height: 44px;
  font-size: 1rem;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
}

@media (max-width: 860px) {
  .product-container {
    grid-template-columns: 1fr;
    gap: 2rem;
    padding: 1.5rem;
  }
}
</style>
