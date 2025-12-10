<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useCartStore } from "@/stores/cartStore";
import { useAuthStore } from "@/stores/authStore";
import { formatPrice } from "@/utils/currency";

const cartStore = useCartStore();
const authStore = useAuthStore();
const currentUserId = authStore.userId;

const cart = computed(() => cartStore.cart);

// Watch for changes in the cart store
import { watch } from "vue";

onMounted(async () => {
  await cartStore.fetchCart(currentUserId);
});

// Watch for changes in the cart store and update local state
watch(
  () => cartStore.cart,
  (newCart) => {
    // This will trigger a re-render when the cart changes
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
  <div class="cart">
    <h1>Your Cart</h1>
    <p v-if="!cart?.items?.length">Your cart is empty</p>
    <div v-else>
      <div
        v-for="item in cart.items"
        :key="item['product-id']"
        class="cart-item"
      >
        <h3>
          <router-link
            :to="`/products/${item['product-id']}`"
            class="product-link"
          >
            {{ item["product-name"] }}
          </router-link>
        </h3>
        <div class="price">{{ formatPrice(item.price, cart.currency) }}</div>
        <div class="quantity-controls">
          <button
            class="quantity-btn"
            :disabled="item.quantity <= 1"
            @click="updateQty(item['product-id'], item.quantity - 1)"
          >
            -
          </button>
          <span class="quantity">{{ item.quantity }}</span>
          <button
            class="quantity-btn"
            @click="updateQty(item['product-id'], item.quantity + 1)"
          >
            +
          </button>
        </div>
        <button class="remove-btn" @click="removeItem(item['product-id'])">
          Remove
        </button>
      </div>
      <div class="summary">
        <div>
          <div class="total-amount">
            Total: {{ formatPrice(cart.total, cart.currency) }}
          </div>
          <p
            class="text-muted"
            style="margin-top: 0.25rem; color: #6c757d; font-size: 0.9rem"
          >
            {{ cart.items.length }} item{{ cart.items.length !== 1 ? "s" : "" }}
            in cart
          </p>
        </div>
        <router-link to="/checkout" class="btn"
          >Proceed to Checkout</router-link
        >
      </div>
    </div>
  </div>
</template>

<style scoped>
.cart {
  max-width: 1000px;
  margin: 0 auto;
  padding: 2rem 1rem;
}

.cart h1 {
  margin-bottom: 2rem;
  color: #2c3e50;
  text-align: center;
}

.cart-item {
  display: grid;
  grid-template-columns: 2fr 1fr 1.5fr 1fr;
  align-items: center;
  gap: 1.5rem;
  padding: 1.25rem;
  border-bottom: 1px solid #eee;
  transition: background-color 0.2s ease;
}

.cart-item:hover {
  background-color: #f9f9f9;
}

.cart-item h3 {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 500;
  color: #2c3e50;
}

.price {
  font-weight: 500;
  color: #4a6fa5;
}

.quantity-controls {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.quantity-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f0f2f5;
  border: 1px solid #ddd;
  border-radius: 4px;
  cursor: pointer;
  font-size: 1rem;
  transition: all 0.2s;
}

.quantity-btn:hover {
  background: #e1e4e8;
}

.quantity {
  min-width: 36px;
  text-align: center;
  font-weight: 500;
}

.remove-btn {
  background: #ffebee;
  color: #c62828;
  border: 1px solid #ffcdd2;
  border-radius: 4px;
  padding: 0.5rem 1rem;
  cursor: pointer;
  font-size: 0.9rem;
  transition: all 0.2s;
  justify-self: end;
}

.remove-btn:hover {
  background: #ffcdd2;
}

.summary {
  margin-top: 2.5rem;
  padding-top: 1.5rem;
  border-top: 2px solid #eee;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.total-amount {
  font-size: 1.5rem;
  font-weight: 600;
  color: #2c3e50;
}

.btn {
  background: #4a6fa5;
  color: white;
  padding: 0.75rem 2rem;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  text-decoration: none;
  font-weight: 500;
  transition: background-color 0.2s;
  text-align: center;
  display: inline-block;
}

.btn:hover {
  background: #3a5a80;
}

/* Responsive adjustments */
@media (max-width: 768px) {
  .cart-item {
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    padding: 1rem 0.5rem;
  }

  .cart-item h3 {
    grid-column: 1 / -1;
  }

  .remove-btn {
    grid-column: 2;
    grid-row: 2;
    justify-self: end;
  }

  .summary {
    flex-direction: column;
    gap: 1.5rem;
    text-align: center;
  }
}
</style>
