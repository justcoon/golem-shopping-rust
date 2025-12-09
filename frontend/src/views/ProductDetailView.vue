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
  return (
    product.value && isProductOnSale(product.value, authStore.pricePreferences)
  );
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
    await cartStore.addItem(
      currentUserId,
      product.value["product-id"],
      qty.value,
    );
  } catch (err) {
    console.error("Error adding to cart:", err);
  } finally {
    isAddingToCart.value = false;
  }
}

async function fetchProduct() {
  if (productId.value) {
    await productStore.fetchProduct(
      productId.value,
      authStore.pricePreferences,
    );
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
  <div class="product-detail">
    <div v-if="isLoading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">
      Error: {{ error.message }}
      <button class="btn btn-outline-primary" @click="fetchProduct">
        Retry
      </button>
    </div>
    <div v-else-if="product" class="product-container">
      <div class="product-gallery">
        <img
          :src="mainImage || getProductImage(product)"
          :alt="product.name"
          class="main-image"
        />
        <div class="thumbnails">
          <img
            v-for="(img, i) in productImages"
            :key="i"
            :src="img"
            :class="{ active: mainImage === img }"
            @click="mainImage = img"
          />
        </div>
      </div>
      <div class="product-info">
        <h1>{{ product.name }}</h1>
        <div class="meta">
          <router-link
            v-if="product.brand"
            :to="{ name: 'products', query: { brand: product.brand } }"
            class="brand-link"
          >
            {{ product.brand }}
          </router-link>
          <span v-else class="brand">No Brand</span>
          <span class="sku">SKU: {{ product["product-id"] }}</span>
        </div>

        <div class="price" :class="{ 'on-sale': hasDiscount }">
          <span class="price">{{
            formatPrice(bestPrice, authStore.pricePreferences.currency)
          }}</span>
          <span v-if="hasDiscount" class="original-price"
            ><span class="price">{{
              formatPrice(originalPrice, authStore.pricePreferences.currency)
            }}</span></span
          >
          <span v-if="hasDiscount" class="discount"
            >{{ discountPercentage }}% OFF</span
          >
        </div>

        <div class="description">
          <h3>Description</h3>
          <p>{{ product.description || "No description available." }}</p>
        </div>

        <div class="actions">
          <div class="quantity">
            <button :disabled="qty <= 1" @click="decreaseQty">-</button>
            <input v-model.number="qty" type="number" min="1" />
            <button @click="increaseQty">+</button>
          </div>

          <button
            class="btn btn-primary"
            :disabled="isAddingToCart"
            @click="addToCart"
          >
            {{ isAddingToCart ? "Adding..." : "Add to Cart" }}
          </button>
        </div>

        <div v-if="product.tags?.length" class="tags">
          <span v-for="tag in product.tags" :key="tag" class="tag">{{
            tag
          }}</span>
        </div>
      </div>
    </div>
    <div v-else class="not-found">
      <h2>Product not found</h2>
      <router-link to="/products" class="btn btn-primary">
        Back to Products
      </router-link>
    </div>
  </div>
</template>

<style scoped>
.product-detail {
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem 1rem;
}

.product-container {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 2rem;
}

.main-image {
  width: 100%;
  height: 400px;
  object-fit: contain;
  margin-bottom: 1rem;
}

.thumbnails {
  display: flex;
  gap: 0.5rem;
}

.thumbnails img {
  width: 60px;
  height: 60px;
  object-fit: cover;
  cursor: pointer;
  border: 2px solid #eee;
  border-radius: 4px;
}

.thumbnails img.active {
  border-color: #4a6fa5;
}

h1 {
  font-size: 2rem;
  margin: 0 0 1rem 0;
}

.meta {
  color: #666;
  margin-bottom: 1rem;
  font-size: 0.9rem;
}

.brand,
.brand-link {
  color: #4a6fa5;
  margin-right: 1rem;
  text-decoration: none;
  font-weight: 500;
  transition: color 0.2s ease;
}

.brand-link:hover {
  color: #3a5a80;
  text-decoration: underline;
}

.price {
  font-size: 1.8rem;
  font-weight: bold;
  color: #333;
  margin: 1rem 0;
}

.price.on-sale {
  color: #dc3545;
}

.original-price {
  text-decoration: line-through;
  color: #999;
  font-size: 1.2rem;
  margin-left: 0.5rem;
}

.discount {
  background: #ffc107;
  color: #000;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  font-size: 0.9rem;
  margin-left: 0.5rem;
}

.description {
  margin: 2rem 0;
  line-height: 1.6;
}

.actions {
  display: flex;
  gap: 1rem;
  margin: 2rem 0;
}

.quantity {
  display: flex;
  align-items: center;
  border: 1px solid #ddd;
  border-radius: 4px;
  overflow: hidden;
}

.quantity button {
  width: 40px;
  height: 40px;
  border: none;
  background: #f8f9fa;
  font-size: 1.2rem;
  cursor: pointer;
}

.quantity input {
  width: 50px;
  height: 40px;
  border: none;
  text-align: center;
  border-left: 1px solid #eee;
  border-right: 1px solid #eee;
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 1rem;
}

.tag {
  background: #e9ecef;
  padding: 0.25rem 0.75rem;
  border-radius: 50px;
  font-size: 0.8rem;
  color: #495057;
}

.loading,
.error,
.not-found {
  text-align: center;
  padding: 4rem 1rem;
}

.error {
  color: #dc3545;
}

.btn {
  padding: 0.5rem 1.5rem;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.2s;
}

.btn-primary {
  background: #4a6fa5;
  color: white;
  border: 1px solid #4a6fa5;
}

.btn-primary:hover {
  background: #3a5a8c;
  border-color: #3a5a8c;
}

.btn-outline-primary {
  background: white;
  color: #4a6fa5;
  border: 1px solid #4a6fa5;
  margin-left: 1rem;
}

.btn-outline-primary:hover {
  background: #f8f9fa;
}
</style>
