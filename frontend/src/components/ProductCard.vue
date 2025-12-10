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

const props = defineProps({
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

const emit = defineEmits(["add-to-cart"]);
</script>

<template>
  <div class="product-card">
    <div class="product-image">
      <img :src="getProductImage(product)" :alt="product.name" />
      <span
        v-if="
          showSaleBadge && isProductOnSale(product, authStore.pricePreferences)
        "
        class="sale-badge"
        >Sale</span
      >
    </div>
    <div class="product-details">
      <h3>
        <router-link
          v-if="!hideLink"
          :to="`/products/${product['product-id']}`"
        >
          {{ product.name }}
        </router-link>
        <template v-else>{{ product.name }}</template>
      </h3>
      <p v-if="!hideBrand" class="brand">{{ product.brand }}</p>
      <div class="price">
        <span
          :class="{
            'sale-price':
              showSaleBadge &&
              isProductOnSale(product, authStore.pricePreferences),
          }"
        >
          ${{ getProductBestPrice(product, authStore.pricePreferences) }}
        </span>
        <span
          v-if="
            showOriginalPrice &&
            isProductOnSale(product, authStore.pricePreferences)
          "
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
          class="btn btn-primary"
          :disabled="isAddingToCart"
          @click="$emit('add-to-cart', product)"
        >
          Add to Cart
        </button>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.product-card {
  border: 1px solid #eee;
  border-radius: 8px;
  overflow: hidden;
  transition: transform 0.2s;
  height: 100%;
  display: flex;
  flex-direction: column;
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
  display: flex;
  flex-direction: column;
  flex-grow: 1;
}

.product-details h3 {
  margin: 0 0 0.5rem 0;
  font-size: 1.1rem;
  flex-grow: 1;
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

.sale-price {
  color: #dc3545;
}

.original-price {
  text-decoration: line-through;
  color: #999;
  font-size: 0.9rem;
  margin-left: 0.5rem;
}

.product-details button {
  margin-top: auto;
  width: 100%;
}
</style>
