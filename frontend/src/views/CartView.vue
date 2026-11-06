<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useCartStore } from "@/stores/cartStore";
import { useAuthStore } from "@/stores/authStore";
import { formatPrice } from "@/utils/currency";
import { getProductImage } from "@/api/services/productService";

const cartStore = useCartStore();
const authStore = useAuthStore();
const currentUserId = authStore.userId;

const cart = computed(() => cartStore.cart);
const isCheckoutActive = computed(() => cartStore.isCheckoutActive);
const activeDraft = computed(() => cartStore.activeDraft);

onMounted(async () => {
  await cartStore.fetchCart(currentUserId);
});

async function cancelActiveCheckout() {
  if (!activeDraft.value) return;
  try {
    await cartStore.cancelCheckout(currentUserId, activeDraft.value.order_id);
  } catch (error) {
    console.error("Failed to cancel checkout:", error);
  }
}

watch(
  () => cartStore.cart,
  (newCart) => {
    console.log("Cart updated:", newCart);
  },
  { deep: true },
);

async function updateQty(productId: string, qty: number) {
  if (qty < 1) return;
  try {
    await cartStore.updateItem(currentUserId, productId, qty);
  } catch (error) {
    console.error("Failed to update item quantity:", error);
  }
}

async function removeItem(productId: string) {
  try {
    await cartStore.removeItem(currentUserId, productId);
  } catch (error) {
    console.error("Failed to remove item:", error);
  }
}
</script>

<template>
  <div class="cart-view">
    <div class="cart-header">
      <h1 class="gradient-text">Your Shopping Cart</h1>
      <p class="subtitle">Review items dynamically held in your persistent Golem Cart Agent</p>
    </div>

    <!-- Empty State -->
    <div v-if="!cart?.items?.length" class="empty-cart glass-card">
      <div class="empty-icon">🛒</div>
      <h2>Your cart is empty</h2>
      <p>Discover agentic products and add them to your cart to get started.</p>
      <router-link to="/products" class="btn btn-primary"> Explore Catalog </router-link>
    </div>

    <!-- Active Checkout Banner -->
    <div v-else-if="isCheckoutActive && activeDraft" class="cart-checkout-banner glass-card">
      <div class="banner-left">
        <span class="banner-icon">⏱️</span>
        <div>
          <h3>Order #{{ activeDraft.order_id }} is in progress</h3>
          <p>Items in your cart are currently locked and reserved. Complete your checkout or cancel the reservation to modify quantities.</p>
        </div>
      </div>
      <div class="banner-actions">
        <router-link to="/checkout" class="btn btn-primary btn-sm">Complete Checkout</router-link>
        <button @click="cancelActiveCheckout" class="btn btn-secondary btn-sm">Cancel Hold</button>
      </div>
    </div>

    <!-- Cart Layout -->
    <div v-if="cart?.items?.length" class="cart-layout">
      <!-- Items List -->
      <div class="cart-items">
        <div v-for="item in cart.items" :key="item.product_id" class="cart-item glass-card">
          <img
            :src="getProductImage({ name: item.product_name })"
            :alt="item.product_name"
            class="item-thumb"
          />

          <div class="item-info">
            <h3 class="item-title">
              <router-link :to="`/products/${item.product_id}`" class="product-link">
                {{ item.product_name }}
              </router-link>
            </h3>
            <div class="item-meta">
              <span class="item-sku">SKU: {{ item.product_id }}</span>
              <span class="unit-price"> {{ formatPrice(item.price, cart.currency) }} each </span>
            </div>
          </div>

          <!-- Quantity Controls -->
          <div class="quantity-picker">
            <button
              class="qty-btn"
              :disabled="isCheckoutActive || item.quantity <= 1"
              @click="updateQty(item.product_id, item.quantity - 1)"
            >
              −
            </button>
            <span class="qty-val">{{ item.quantity }}</span>
            <button
              class="qty-btn"
              :disabled="isCheckoutActive"
              @click="updateQty(item.product_id, item.quantity + 1)"
            >
              +
            </button>
          </div>

          <!-- Line Total -->
          <div class="item-total">
            {{ formatPrice(item.price * item.quantity, cart.currency) }}
          </div>

          <!-- Remove Button -->
          <button
            class="remove-btn"
            title="Remove item"
            :disabled="isCheckoutActive"
            @click="removeItem(item.product_id)"
          >
            ✕
          </button>
        </div>
      </div>

      <!-- Order Summary Card -->
      <div class="summary-col">
        <div class="summary-card glass-card">
          <h2 class="summary-title">Summary</h2>

          <div class="summary-rows">
            <div class="summary-row">
              <span>Items ({{ cart.items.length }})</span>
              <span class="val">{{ formatPrice(cart.total, cart.currency) }}</span>
            </div>
            <div class="summary-row">
              <span>Estimated Shipping</span>
              <span class="free-pill">FREE</span>
            </div>
          </div>

          <div class="summary-divider"></div>

          <div class="summary-total">
            <span>Total</span>
            <span class="total-amount">
              {{ formatPrice(cart.total, cart.currency) }}
            </span>
          </div>

          <router-link to="/checkout" class="btn btn-primary checkout-btn">
            Proceed to Checkout →
          </router-link>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cart-view {
  max-width: 1100px;
  margin: 0 auto;
  padding: 1.5rem 1rem 3rem;
}

.cart-header {
  margin-bottom: 2rem;
}

.cart-header h1 {
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 0.4rem;
}

.subtitle {
  color: var(--text-muted);
  font-size: 0.95rem;
}

.empty-cart {
  text-align: center;
  padding: 4rem 2rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
}

.empty-icon {
  font-size: 3.5rem;
  opacity: 0.8;
}

.empty-cart h2 {
  font-size: 1.5rem;
  color: var(--text-main);
  margin: 0;
}

.empty-cart p {
  color: var(--text-muted);
  margin: 0 0 1rem;
}

/* Layout */
.cart-layout {
  display: grid;
  grid-template-columns: 1fr 340px;
  gap: 2rem;
  align-items: start;
}

.cart-items {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.cart-item {
  display: grid;
  grid-template-columns: 80px 1fr auto auto auto;
  align-items: center;
  gap: 1.25rem;
  padding: 1.25rem;
  transition: all 0.2s ease;
}

.cart-item:hover {
  border-color: rgba(255, 255, 255, 0.15);
}

.item-thumb {
  width: 80px;
  height: 80px;
  object-fit: cover;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: rgba(15, 23, 42, 0.6);
}

.item-info {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.item-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.product-link {
  color: var(--text-main);
  text-decoration: none;
  transition: color 0.2s;
}

.product-link:hover {
  color: #c084fc;
}

.item-meta {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-size: 0.8rem;
  color: var(--text-muted);
}

.item-sku {
  font-family: "JetBrains Mono", monospace;
}

.unit-price {
  font-family: "JetBrains Mono", monospace;
  color: var(--accent-cyan);
}

/* Quantity controls */
.quantity-picker {
  display: flex;
  align-items: center;
  background: rgba(15, 23, 42, 0.8);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow: hidden;
}

.qty-btn {
  width: 32px;
  height: 32px;
  background: transparent;
  border: none;
  color: var(--text-main);
  font-size: 1rem;
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

.qty-val {
  min-width: 32px;
  text-align: center;
  font-family: "JetBrains Mono", monospace;
  font-size: 0.9rem;
  font-weight: 600;
}

.item-total {
  font-family: "JetBrains Mono", monospace;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--accent-cyan);
  min-width: 80px;
  text-align: right;
}

.remove-btn {
  background: rgba(239, 68, 68, 0.1);
  color: #f87171;
  border: 1px solid rgba(239, 68, 68, 0.2);
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s;
  font-size: 0.9rem;
}

.remove-btn:hover {
  background: rgba(239, 68, 68, 0.25);
  border-color: #f87171;
  box-shadow: 0 0 10px rgba(239, 68, 68, 0.3);
}

/* Summary Card */
.summary-card {
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  position: sticky;
  top: 90px;
}

.summary-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin: 0;
  color: var(--text-main);
}

.summary-rows {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.summary-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.9rem;
  color: var(--text-muted);
}

.summary-row .val {
  font-family: "JetBrains Mono", monospace;
  color: var(--text-main);
  font-weight: 500;
}

.free-pill {
  font-size: 0.75rem;
  font-weight: 700;
  color: #34d399;
  background: rgba(52, 211, 153, 0.15);
  padding: 0.15rem 0.5rem;
  border-radius: 9999px;
  border: 1px solid rgba(52, 211, 153, 0.3);
}

.summary-divider {
  height: 1px;
  background: var(--border-color);
}

.summary-total {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--text-main);
}

.total-amount {
  font-family: "JetBrains Mono", monospace;
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--accent-cyan);
  text-shadow: 0 0 16px rgba(6, 182, 212, 0.3);
}

.checkout-btn {
  width: 100%;
  padding: 0.85rem;
  font-size: 1rem;
  font-weight: 600;
  text-align: center;
  text-decoration: none;
  display: block;
}

.cart-checkout-banner {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.5rem;
  margin-bottom: 2rem;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.12), rgba(168, 85, 247, 0.12));
  border: 1px solid rgba(129, 140, 248, 0.3);
  border-radius: 12px;
  gap: 1.5rem;
}

.cart-checkout-banner .banner-left {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.cart-checkout-banner .banner-icon {
  font-size: 1.8rem;
}

.cart-checkout-banner h3 {
  margin: 0 0 0.2rem;
  font-size: 1.05rem;
  color: var(--text-primary, #fff);
}

.cart-checkout-banner p {
  margin: 0;
  font-size: 0.85rem;
  color: var(--text-muted);
}

.cart-checkout-banner .banner-actions {
  display: flex;
  gap: 0.75rem;
  flex-shrink: 0;
}

.security-note {
  text-align: center;
  font-size: 0.75rem;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
}

@media (max-width: 860px) {
  .cart-layout {
    grid-template-columns: 1fr;
  }

  .cart-item {
    grid-template-columns: 60px 1fr auto;
    grid-template-areas:
      "thumb info remove"
      "thumb qty total";
    gap: 0.75rem;
  }

  .item-thumb {
    grid-area: thumb;
    width: 60px;
    height: 60px;
  }

  .item-info {
    grid-area: info;
  }

  .remove-btn {
    grid-area: remove;
    justify-self: end;
  }

  .quantity-picker {
    grid-area: qty;
  }

  .item-total {
    grid-area: total;
    justify-self: end;
  }
}
</style>
