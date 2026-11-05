<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from "vue";
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

const isCheckoutActive = computed(() => cartStore.isCheckoutActive);
const activeDraft = computed(() => cartStore.activeDraft);

// Timer for reservation countdown
const secondsRemaining = ref<number>(0);
let timerInterval: ReturnType<typeof setInterval> | null = null;

function updateCountdown() {
  if (!activeDraft.value) {
    secondsRemaining.value = 0;
    return;
  }
  const exp = new Date(activeDraft.value.expires_at).getTime();
  const diff = Math.max(0, Math.floor((exp - Date.now()) / 1000));
  secondsRemaining.value = diff;
}

const formattedCountdown = computed(() => {
  const mins = Math.floor(secondsRemaining.value / 60);
  const secs = secondsRemaining.value % 60;
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
});

watch(
  activeDraft,
  (draft) => {
    if (draft) {
      updateCountdown();
      if (!timerInterval) {
        timerInterval = setInterval(updateCountdown, 1000);
      }
    } else {
      if (timerInterval) {
        clearInterval(timerInterval);
        timerInterval = null;
      }
      secondsRemaining.value = 0;
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  if (timerInterval) {
    clearInterval(timerInterval);
    timerInterval = null;
  }
});

// Shipping address
const shipping = ref<Address & { email: string }>({
  email: "",
  street: "",
  city: "",
  state_or_region: "",
  country: "",
  postal_code: "",
  name: "",
  phone_number: "",
});

// Billing address
const billing = ref<Address>({ ...shipping.value });

watch(
  shipping,
  (newShipping) => {
    if (sameAsBilling.value) {
      billing.value = { ...newShipping };
    }
  },
  { deep: true },
);

watch(sameAsBilling, (isSame) => {
  if (isSame) {
    billing.value = { ...shipping.value };
  }
});

onMounted(() => {
  if (!cart.value) cartStore.fetchCart(currentUserId);

  if (cart.value?.shipping_address) {
    shipping.value = {
      email: cart.value.email || "",
      ...cart.value.shipping_address,
    };
  } else if (cart.value?.email) {
    shipping.value.email = cart.value.email;
  }

  if (cart.value?.billing_address) {
    billing.value = { ...cart.value.billing_address };
    sameAsBilling.value = false;
  }
});

async function prepareOrder() {
  if (!cart.value) return;

  isSubmitting.value = true;

  try {
    if (shipping.value.email) {
      await cartStore.updateEmail(currentUserId, shipping.value.email);
    }

    const { email: _shippingEmail, ...shippingAddressOnly } = shipping.value;
    const { email: _billingEmail, ...billingAddressOnly } = billing.value;

    await cartStore.updateShippingAddress(currentUserId, shippingAddressOnly);
    await cartStore.updateBillingAddress(currentUserId, billingAddressOnly);

    await cartStore.prepareCheckout(currentUserId);
  } catch (error) {
    console.error("Prepare checkout failed:", error);
  } finally {
    isSubmitting.value = false;
  }
}

async function confirmOrder() {
  if (!activeDraft.value) return;

  isSubmitting.value = true;
  try {
    const order = await cartStore.confirmCheckout(currentUserId, activeDraft.value.order_id);
    if (order && order.order_id) {
      await cartStore.clearCart();
      await router.push({
        name: "order-detail",
        params: { id: order.order_id },
      });
    } else {
      await router.push({ name: "orders" });
    }
  } catch (error) {
    console.error("Confirm checkout failed:", error);
  } finally {
    isSubmitting.value = false;
  }
}

async function cancelOrder() {
  if (!activeDraft.value) return;

  isSubmitting.value = true;
  try {
    await cartStore.cancelCheckout(currentUserId, activeDraft.value.order_id);
  } catch (error) {
    console.error("Cancel checkout failed:", error);
  } finally {
    isSubmitting.value = false;
  }
}

function handleFormSubmit() {
  if (isCheckoutActive.value) {
    confirmOrder();
  } else {
    prepareOrder();
  }
}
</script>

<template>
  <div class="checkout-view">
    <div class="checkout-header">
      <h1 class="gradient-text">Complete Your Order</h1>
      <p class="subtitle">Secure multi-agent checkout and durable settlement</p>
    </div>

    <!-- Empty Cart Fallback -->
    <div v-if="!cart?.items?.length" class="empty-state glass-card">
      <div class="empty-icon">🛒</div>
      <h2>Your cart is empty</h2>
      <p>Add items to your cart before proceeding to checkout.</p>
      <router-link to="/products" class="btn btn-primary">Shop Now</router-link>
    </div>

    <!-- Active Reservation Banner -->
    <div v-if="cart?.items?.length && isCheckoutActive && activeDraft" class="reservation-banner glass-card">
      <div class="banner-left">
        <span class="timer-icon">⏱️</span>
        <div>
          <h3>Order #{{ activeDraft.order_id }} is Reserved</h3>
          <p>Inventory held exclusively for you. Please review and confirm your order before the hold expires.</p>
        </div>
      </div>
      <div class="banner-right">
        <span class="countdown-badge">{{ formattedCountdown }}</span>
      </div>
    </div>

    <!-- Checkout Grid -->
    <div v-if="cart?.items?.length" class="checkout-grid">
      <!-- Checkout Form -->
      <form class="checkout-form glass-card" @submit.prevent="handleFormSubmit">
        <div class="form-section-title">
          <span class="icon">📦</span>
          <h2>Shipping Information</h2>
        </div>

        <div class="form-group">
          <label for="shipping-email">Email Address</label>
          <input
            id="shipping-email"
            v-model="shipping.email"
            type="email"
            placeholder="your@email.com"
            required
            class="input-dark"
          />
        </div>

        <div class="form-group">
          <label for="shipping-name">Full Name</label>
          <input
            id="shipping-name"
            v-model="shipping.name"
            placeholder="Ada Lovelace"
            required
            class="input-dark"
          />
        </div>

        <div class="form-group">
          <label for="shipping-street">Street Address</label>
          <input
            id="shipping-street"
            v-model="shipping.street"
            placeholder="123 Quantum Way"
            required
            class="input-dark"
          />
        </div>

        <div class="form-row">
          <div class="form-group">
            <label for="shipping-city">City</label>
            <input
              id="shipping-city"
              v-model="shipping.city"
              placeholder="San Francisco"
              required
              class="input-dark"
            />
          </div>
          <div class="form-group">
            <label for="shipping-region">State / Region</label>
            <input
              id="shipping-region"
              v-model="shipping.state_or_region"
              placeholder="CA"
              required
              class="input-dark"
            />
          </div>
        </div>

        <div class="form-row">
          <div class="form-group">
            <label for="shipping-postal-code">Postal Code</label>
            <input
              id="shipping-postal-code"
              v-model="shipping.postal_code"
              placeholder="94103"
              required
              class="input-dark"
            />
          </div>
          <div class="form-group">
            <label for="shipping-country">Country</label>
            <input
              id="shipping-country"
              v-model="shipping.country"
              placeholder="United States"
              required
              class="input-dark"
            />
          </div>
        </div>

        <div class="form-group">
          <label for="shipping-phone">Phone Number</label>
          <input
            id="shipping-phone"
            v-model="shipping.phone_number"
            placeholder="+1 (555) 019-2834"
            class="input-dark"
          />
        </div>

        <!-- Same as billing checkbox -->
        <label class="custom-checkbox">
          <input id="same-as-billing" v-model="sameAsBilling" type="checkbox" />
          <span class="checkmark"></span>
          <span class="label-text">Billing address is identical to shipping address</span>
        </label>

        <!-- Billing Info (if different) -->
        <div v-if="!sameAsBilling" class="billing-section">
          <div class="form-section-title">
            <span class="icon">💳</span>
            <h2>Billing Information</h2>
          </div>

          <div class="form-group">
            <label for="billing-name">Full Name</label>
            <input
              id="billing-name"
              v-model="billing.name"
              placeholder="Ada Lovelace"
              class="input-dark"
            />
          </div>

          <div class="form-group">
            <label for="billing-street">Street Address</label>
            <input
              id="billing-street"
              v-model="billing.street"
              placeholder="123 Quantum Way"
              class="input-dark"
            />
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="billing-city">City</label>
              <input
                id="billing-city"
                v-model="billing.city"
                placeholder="San Francisco"
                class="input-dark"
              />
            </div>
            <div class="form-group">
              <label for="billing-region">State / Region</label>
              <input
                id="billing-region"
                v-model="billing.state_or_region"
                placeholder="CA"
                class="input-dark"
              />
            </div>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="billing-postal-code">Postal Code</label>
              <input
                id="billing-postal-code"
                v-model="billing.postal_code"
                placeholder="94103"
                class="input-dark"
              />
            </div>
            <div class="form-group">
              <label for="billing-country">Country</label>
              <input
                id="billing-country"
                v-model="billing.country"
                placeholder="United States"
                class="input-dark"
              />
            </div>
          </div>

          <div class="form-group">
            <label for="billing-phone">Phone Number</label>
            <input
              id="billing-phone"
              v-model="billing.phone_number"
              placeholder="+1 (555) 019-2834"
              class="input-dark"
            />
          </div>
        </div>

        <div class="action-buttons-group">
          <button type="submit" :disabled="isSubmitting" class="btn btn-primary submit-btn">
            <span v-if="isSubmitting" class="spinner"></span>
            <span v-else-if="isCheckoutActive">
              Confirm & Place Order ({{ formatPrice(cart.total, cart.currency) }})
            </span>
            <span v-else>
              Review & Reserve Items ({{ formatPrice(cart.total, cart.currency) }})
            </span>
          </button>
          <button
            v-if="isCheckoutActive"
            type="button"
            :disabled="isSubmitting"
            @click="cancelOrder"
            class="btn btn-secondary cancel-draft-btn"
          >
            Cancel Reservation
          </button>
        </div>
      </form>

      <!-- Order Summary Card -->
      <div class="order-summary-col">
        <div class="summary-card glass-card">
          <h2 class="summary-title">Order Summary</h2>

          <div class="order-items">
            <div v-for="item in cart.items" :key="item.product_id" class="order-item">
              <div class="item-details">
                <router-link :to="`/products/${item.product_id}`" class="item-name">
                  {{ item.product_name }}
                </router-link>
                <div class="item-meta">
                  <span class="item-qty">Qty: {{ item.quantity }}</span>
                  <span class="item-unit-price">
                    {{ formatPrice(item.price, cart.currency) }} ea
                  </span>
                </div>
              </div>
              <div class="item-line-total">
                {{ formatPrice(item.price * item.quantity, cart.currency) }}
              </div>
            </div>
          </div>

          <div class="summary-divider"></div>

          <div class="totals-section">
            <div class="total-row">
              <span>Subtotal</span>
              <span class="val">{{ formatPrice(cart.total, cart.currency) }}</span>
            </div>
            <div class="total-row">
              <span>Standard Shipping</span>
              <span class="free-pill">FREE</span>
            </div>
            <div class="total-row grand-total">
              <span>Total Amount</span>
              <span class="total-highlight">
                {{ formatPrice(cart.total, cart.currency) }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.checkout-view {
  max-width: 1100px;
  margin: 0 auto;
  padding: 1.5rem 1rem 3rem;
}

.checkout-header {
  margin-bottom: 2rem;
}

.checkout-header h1 {
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 0.4rem;
}

.subtitle {
  color: var(--text-muted);
  font-size: 0.95rem;
}

.reservation-banner {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.25rem 1.5rem;
  margin-bottom: 2rem;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.15), rgba(168, 85, 247, 0.15));
  border: 1px solid rgba(129, 140, 248, 0.35);
  border-radius: 12px;
}

.banner-left {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.timer-icon {
  font-size: 2rem;
}

.banner-left h3 {
  margin: 0 0 0.25rem;
  font-size: 1.1rem;
  color: var(--text-primary, #fff);
}

.banner-left p {
  margin: 0;
  font-size: 0.88rem;
  color: var(--text-muted);
}

.countdown-badge {
  font-family: monospace;
  font-size: 1.4rem;
  font-weight: 700;
  background: rgba(0, 0, 0, 0.4);
  padding: 0.4rem 0.8rem;
  border-radius: 8px;
  color: #fbbf24;
  border: 1px solid rgba(251, 191, 36, 0.3);
}

.action-buttons-group {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.cancel-draft-btn {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: #f87171;
  padding: 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  font-weight: 600;
  transition: all 0.2s ease;
}

.cancel-draft-btn:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.5);
}

.empty-state {
  text-align: center;
  padding: 4rem 2rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
}

.empty-icon {
  font-size: 3rem;
}

.checkout-grid {
  display: grid;
  grid-template-columns: 1.6fr 1fr;
  gap: 2rem;
  align-items: start;
}

/* Form */
.checkout-form {
  padding: 2rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.form-section-title {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 0.5rem;
}

.form-section-title .icon {
  font-size: 1.3rem;
}

.form-section-title h2 {
  font-size: 1.25rem;
  font-weight: 600;
  margin: 0;
  color: var(--text-main);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.form-group label {
  font-size: 0.85rem;
  font-weight: 500;
  color: var(--text-muted);
}

.input-dark {
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  color: var(--text-main);
  font-size: 0.95rem;
  outline: none;
  transition: all 0.2s;
}

.input-dark:focus {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.2);
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}

/* Custom Checkbox */
.custom-checkbox {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  cursor: pointer;
  user-select: none;
  font-size: 0.9rem;
  color: var(--text-main);
  padding: 0.5rem 0;
}

.custom-checkbox input {
  display: none;
}

.checkmark {
  width: 20px;
  height: 20px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: rgba(15, 23, 42, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.custom-checkbox input:checked + .checkmark {
  background: var(--primary);
  border-color: var(--primary);
}

.custom-checkbox input:checked + .checkmark::after {
  content: "✔";
  color: white;
  font-size: 0.75rem;
  font-weight: bold;
}

.billing-section {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  margin-top: 1rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--border-color);
}

.submit-btn {
  margin-top: 1rem;
  padding: 1rem;
  font-size: 1.05rem;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Summary Column */
.summary-card {
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  position: sticky;
  top: 90px;
}

.summary-title {
  font-size: 1.2rem;
  font-weight: 700;
  margin: 0;
  color: var(--text-main);
}

.order-items {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-height: 260px;
  overflow-y: auto;
  padding-right: 0.25rem;
}

.order-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  padding-bottom: 0.75rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.item-details {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.item-name {
  color: var(--text-main);
  text-decoration: none;
  font-size: 0.9rem;
  font-weight: 500;
  transition: color 0.2s;
}

.item-name:hover {
  color: #c084fc;
}

.item-meta {
  display: flex;
  gap: 0.6rem;
  font-size: 0.75rem;
  color: var(--text-muted);
}

.item-unit-price {
  font-family: "JetBrains Mono", monospace;
}

.item-line-total {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--accent-cyan);
}

.summary-divider {
  height: 1px;
  background: var(--border-color);
}

.totals-section {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.total-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.9rem;
  color: var(--text-muted);
}

.total-row .val {
  font-family: "JetBrains Mono", monospace;
  color: var(--text-main);
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

.grand-total {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--text-main);
  padding-top: 0.75rem;
  border-top: 1px solid var(--border-color);
  margin-top: 0.4rem;
}

.total-highlight {
  font-family: "JetBrains Mono", monospace;
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--accent-cyan);
  text-shadow: 0 0 16px rgba(6, 182, 212, 0.3);
}

.guarantee-box {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  background: rgba(139, 92, 246, 0.1);
  border: 1px solid rgba(139, 92, 246, 0.2);
  font-size: 0.8rem;
  color: var(--text-muted);
}

.guarantee-box strong {
  display: block;
  color: #c084fc;
  font-size: 0.85rem;
  margin-bottom: 0.2rem;
}

.guarantee-box p {
  margin: 0;
  line-height: 1.4;
}

@media (max-width: 860px) {
  .checkout-grid {
    grid-template-columns: 1fr;
  }
}
</style>
