<script setup lang="ts">
import { useAuthStore } from "@/stores/authStore";
import { formatPrice } from "@/utils/currency";
import {
  Product,
  getProductBestPrice,
  isProductOnSale,
  getProductOriginalPrice,
  getProductImage,
} from "@/api/services/productService";

const authStore = useAuthStore();

defineProps({
  product: {
    type: Object as () => Product,
    required: true,
  },
  isAddingToCart: {
    type: Boolean,
    default: false,
  },
  showSaleBadge: {
    type: Boolean,
    default: true,
  },
  showOriginalPrice: {
    type: Boolean,
    default: true,
  },
  showAddToCart: {
    type: Boolean,
    default: true,
  },
  hideBrand: {
    type: Boolean,
    default: false,
  },
  hideLink: {
    type: Boolean,
    default: false,
  },
});

defineEmits(["add-to-cart"]);
</script>

<template>
  <div class="product-card glass-card">
    <div class="product-image">
      <img :src="getProductImage(product)" :alt="product.name" loading="lazy" />
      <span
        v-if="showSaleBadge && isProductOnSale(product, authStore.pricePreferences)"
        class="sale-badge"
      >
        SALE
      </span>
      <span v-if="product.brand && !hideBrand" class="brand-pill">
        {{ product.brand }}
      </span>
    </div>

    <div class="product-details">
      <h3 class="product-title">
        <router-link v-if="!hideLink" :to="`/products/${product.product_id}`">
          {{ product.name }}
        </router-link>
        <template v-else>{{ product.name }}</template>
      </h3>

      <div class="price-row">
        <span class="price-val">
          ${{ getProductBestPrice(product, authStore.pricePreferences) }}
        </span>
        <span
          v-if="showOriginalPrice && isProductOnSale(product, authStore.pricePreferences)"
          class="original-price"
        >
          {{
            formatPrice(
              getProductOriginalPrice(product, authStore.pricePreferences),
              authStore.pricePreferences.currency,
            )
          }}
        </span>
      </div>

      <slot>
        <button
          v-if="showAddToCart"
          class="btn btn-primary btn-add"
          :disabled="isAddingToCart"
          @click="$emit('add-to-cart', product)"
        >
          <span v-if="isAddingToCart" class="spinner"></span>
          <span v-else>+ Add to Cart</span>
        </button>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.product-card {
  width: 100%;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.product-image {
  position: relative;
  height: 200px;
  background: rgba(11, 15, 25, 0.7);
  overflow: hidden;
  border-bottom: 1px solid var(--border-color);
}

.product-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: transform 0.3s ease;
}

.product-card:hover .product-image img {
  transform: scale(1.05);
}

.sale-badge {
  position: absolute;
  top: 10px;
  right: 10px;
  background: linear-gradient(135deg, #ec4899 0%, #ef4444 100%);
  color: white;
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 0.7rem;
  font-weight: 800;
  letter-spacing: 0.05em;
  box-shadow: 0 2px 10px rgba(236, 72, 153, 0.5);
}

.brand-pill {
  position: absolute;
  bottom: 8px;
  left: 10px;
  background: rgba(11, 15, 25, 0.85);
  backdrop-filter: blur(8px);
  border: 1px solid var(--border-color);
  color: #c084fc;
  padding: 2px 8px;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
}

.product-details {
  padding: 16px;
  display: flex;
  flex-direction: column;
  flex-grow: 1;
  gap: 12px;
}

.product-title {
  font-size: 1rem;
  font-weight: 600;
  line-height: 1.4;
  margin: 0;
  flex-grow: 1;
}

.product-title a {
  color: var(--text-main);
  text-decoration: none;
  transition: color 0.2s;
}

.product-title a:hover {
  color: #38bdf8;
}

.price-row {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.price-val {
  font-size: 1.25rem;
  font-weight: 700;
  color: #38bdf8;
  font-family: var(--font-mono);
}

.original-price {
  text-decoration: line-through;
  color: var(--text-dim);
  font-size: 0.88rem;
  font-family: var(--font-mono);
}

.btn-add {
  width: 100%;
  margin-top: auto;
  font-size: 0.88rem;
  padding: 9px 14px;
}
</style>
