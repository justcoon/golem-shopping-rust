<script setup lang="ts">
import { ref, onMounted, watch, computed } from "vue";
import { useRouter } from "vue-router";
import { useCartStore } from "@/stores/cartStore";
import { useAuthStore } from "@/stores/authStore";
import type { Address } from "@/types/address";
import { formatPrice } from "@/utils/currency";

const cartStore = useCartStore();
const authStore = useAuthStore();
const router = useRouter();
const currentUserId = authStore.userId;

const cart = computed(() => cartStore.cart);
const isSubmitting = ref(false);
const sameAsBilling = ref(true);

// Shipping address
const shipping = ref<Address & { email: string }>({
  email: "",
  street: "",
  city: "",
  "state-or-region": "",
  country: "",
  "postal-code": "",
  name: "",
  "phone-number": "",
});

// Billing address (starts as a copy of shipping address)
const billing = ref<Address>({ ...shipping.value });

// When shipping address changes, update billing address if sameAsBilling is true
watch(
  shipping,
  (newShipping) => {
    if (sameAsBilling.value) {
      billing.value = { ...newShipping };
    }
  },
  { deep: true },
);

// When sameAsBilling changes, update billing address if needed
watch(sameAsBilling, (isSame) => {
  if (isSame) {
    billing.value = { ...shipping.value };
  }
});

// const payment = ref({
//   card: '',
//   expiry: '',
//   cvc: ''
// });

onMounted(() => {
  if (!cart.value) cartStore.fetchCart(currentUserId);

  // If we have a saved cart with addresses, populate the form
  if (cart.value?.["shipping-address"]) {
    shipping.value = { ...cart.value["shipping-address"] };
  }

  if (cart.value?.["billing-address"]) {
    billing.value = { ...cart.value["billing-address"] };
    sameAsBilling.value = false;
  }
});

async function submitOrder() {
  if (!cart.value) return;

  isSubmitting.value = true;

  try {
    // Update cart with email first
    if (shipping.value.email) {
      await cartStore.updateEmail(currentUserId, shipping.value.email);
    }

    // Update cart with addresses
    await cartStore.updateShippingAddress(currentUserId, shipping.value);
    await cartStore.updateBillingAddress(currentUserId, billing.value);

    // Create order and get the order ID from the response
    const order = await cartStore.checkout(currentUserId);

    // Redirect to order confirmation
    if (order && order["order-id"]) {
      // Clear cart
      await cartStore.clearCart();

      await router.push({
        name: "order-detail",
        params: { id: order["order-id"] },
      });
    } else {
      // If for some reason we don't have the order, go to orders page
      await router.push({ name: "orders" });
    }
  } catch (error) {
    console.error("Checkout failed:", error);
  } finally {
    isSubmitting.value = false;
  }
}
</script>

<template>
  <div class="checkout">
    <h1>Checkout</h1>

    <div v-if="!cart?.items?.length">
      <p>Your cart is empty</p>
      <router-link to="/products" class="btn">Shop Now</router-link>
    </div>

    <div v-else class="checkout-grid">
      <form class="checkout-form" @submit.prevent="submitOrder">
        <h2>Shipping Information</h2>
        <div class="form-group">
          <label for="shipping-email">Email Address</label>
          <input
            id="shipping-email"
            v-model="shipping.email"
            type="email"
            placeholder="your@email.com"
            required
          />
        </div>
        <div class="form-group">
          <label for="shipping-name">Full Name</label>
          <input
            id="shipping-name"
            v-model="shipping.name"
            placeholder="John Doe"
            required
          />
        </div>

        <div class="form-group">
          <label for="shipping-street">Street Address</label>
          <input
            id="shipping-street"
            v-model="shipping.street"
            placeholder="123 Main St"
            required
          />
        </div>

        <div class="form-row">
          <div class="form-group">
            <label for="shipping-city">City</label>
            <input
              id="shipping-city"
              v-model="shipping.city"
              placeholder="New York"
              required
            />
          </div>
          <div class="form-group">
            <label for="shipping-region">State/Region</label>
            <input
              id="shipping-region"
              v-model="shipping['state-or-region']"
              placeholder="NY"
              required
            />
          </div>
        </div>

        <div class="form-row">
          <div class="form-group">
            <label for="shipping-postal-code">Postal Code</label>
            <input
              id="shipping-postal-code"
              v-model="shipping['postal-code']"
              placeholder="10001"
              required
            />
          </div>
          <div class="form-group">
            <label for="shipping-country">Country</label>
            <input
              id="shipping-country"
              v-model="shipping.country"
              placeholder="United States"
              required
            />
          </div>
        </div>

        <div class="form-group">
          <label for="shipping-phone">Phone Number</label>
          <input
            id="shipping-phone"
            v-model="shipping['phone-number']"
            placeholder="+1 (555) 123-4567"
          />
        </div>

        <div class="form-checkbox">
          <input id="same-as-billing" v-model="sameAsBilling" type="checkbox" />
          <label for="same-as-billing"
            >Billing address is the same as shipping</label
          >
        </div>

        <div v-if="!sameAsBilling">
          <h2>Billing Information</h2>
          <div class="form-group">
            <label for="billing-name">Full Name</label>
            <input
              id="billing-name"
              v-model="billing.name"
              placeholder="John Doe"
            />
          </div>

          <div class="form-group">
            <label for="billing-street">Street Address</label>
            <input
              id="billing-street"
              v-model="billing.street"
              placeholder="123 Main St"
            />
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="billing-city">City</label>
              <input
                id="billing-city"
                v-model="billing.city"
                placeholder="New York"
              />
            </div>
            <div class="form-group">
              <label for="billing-region">State/Region</label>
              <input
                id="billing-region"
                v-model="billing['state-or-region']"
                placeholder="NY"
              />
            </div>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="billing-postal-code">Postal Code</label>
              <input
                id="billing-postal-code"
                v-model="billing['postal-code']"
                placeholder="10001"
              />
            </div>
            <div class="form-group">
              <label for="billing-country">Country</label>
              <input
                id="billing-country"
                v-model="billing.country"
                placeholder="United States"
              />
            </div>
          </div>

          <div class="form-group">
            <label for="billing-phone">Phone Number</label>
            <input
              id="billing-phone"
              v-model="billing['phone-number']"
              placeholder="+1 (555) 123-4567"
            />
          </div>
        </div>

        <button type="submit" :disabled="isSubmitting" class="btn">
          {{ isSubmitting ? "Processing..." : `Pay $${cart.total.toFixed(2)}` }}
        </button>
      </form>

      <div class="order-summary">
        <h2>Order Summary</h2>
        <div class="order-items">
          <div
            v-for="item in cart.items"
            :key="item['product-id']"
            class="order-item"
          >
            <div class="item-details">
              <h6 class="item-name">
                <router-link
                  :to="`/products/${item['product-id']}`"
                  class="product-link"
                >
                  {{ item["product-name"] }}
                </router-link>
              </h6>
              <div class="item-meta">
                <span class="item-quantity">Qty: {{ item.quantity }}</span>
              </div>
              <div class="item-price">
                {{ formatPrice(item.price, cart.currency) }} each
              </div>
            </div>
          </div>
        </div>

        <div class="order-totals">
          <div class="total">
            <span>Total</span>
            <span>{{ formatPrice(cart.total, cart.currency) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.checkout {
  max-width: 1000px;
  margin: 0 auto;
  padding: 1rem;
}

.checkout-grid {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 2rem;
  margin-top: 2rem;
}

input {
  width: 100%;
  padding: 0.5rem;
  margin-bottom: 1rem;
  border: 1px solid #ddd;
  border-radius: 4px;
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}

.order-summary {
  background: #f8f9fa;
  padding: 1.5rem;
  border-radius: 8px;
  height: fit-content;
}

.order-item {
  display: flex;
  justify-content: space-between;
  padding: 0.5rem 0;
  border-bottom: 1px solid #eee;
}

.order-total {
  font-weight: bold;
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 2px solid #ddd;
  display: flex;
  justify-content: space-between;
}

.btn {
  background: #4a6fa5;
  color: white;
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  width: 100%;
  font-size: 1rem;
  margin-top: 1rem;
}

.btn:disabled {
  background: #ccc;
  cursor: not-allowed;
}
</style>
