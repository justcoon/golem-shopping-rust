<script setup lang="ts">
import { computed, watch, onMounted } from "vue";
import { useRoute } from "vue-router";
import { useOrderStore } from "@/stores/orderStore";
import { getProductImage } from "@/api/services/productService";
import { DateTime, formatDate as formatDateTime } from "@/types/datetime.ts";
import { formatPrice } from "@/utils/currency";

const route = useRoute();
const orderStore = useOrderStore();

const order = computed(() => orderStore.currentOrder);
const isLoading = computed(() => orderStore.isLoading);
const error = computed(() => orderStore.error);

const orderStatuses = [
  { value: "PROCESSING", label: "Order Placed & Processing" },
  { value: "SHIPPED", label: "Shipped & In Transit" },
  { value: "DELIVERED", label: "Delivered" },
];

function formatDate(d: DateTime) {
  return formatDateTime(d, {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatStatus(status: string) {
  return status
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(" ");
}

function getStatusClass(status: string) {
  switch (status) {
    case "PROCESSING":
      return "status-processing";
    case "SHIPPED":
      return "status-shipped";
    case "DELIVERED":
      return "status-delivered";
    case "CANCELLED":
      return "status-cancelled";
    default:
      return "status-default";
  }
}

function isStatusActive(status: string) {
  if (!order.value) return false;
  const curr = order.value.order_status.toUpperCase();
  if (status === "PROCESSING" && (curr === "NEW" || curr === "PROCESSING")) return true;
  return curr === status;
}

function isStatusCompleted(status: string) {
  if (!order.value) return false;
  const statusOrder = ["NEW", "SHIPPED", "DELIVERED"];
  const rawCurr = order.value.order_status.toUpperCase();
  const curr = rawCurr === "PROCESSING" ? "NEW" : rawCurr;
  const target = status === "PROCESSING" ? "NEW" : status;
  const currentStatusIndex = statusOrder.indexOf(curr);
  const statusIndex = statusOrder.indexOf(target);
  return currentStatusIndex >= statusIndex && currentStatusIndex !== -1;
}

function getStatusDate() {
  if (!order.value) return null;
  return order.value.updated_at;
}

async function fetchOrder() {
  const orderId = route.params.id as string;
  if (orderId) {
    await orderStore.fetchOrder(orderId);
  }
}

onMounted(fetchOrder);
watch(() => route.params.id, fetchOrder);
</script>

<template>
  <div class="order-detail-view">
    <div class="back-nav">
      <router-link to="/orders" class="btn-back"> <span>←</span> Back to All Orders </router-link>
    </div>

    <!-- Loading -->
    <div v-if="isLoading" class="state-card glass-card">
      <div class="spinner"></div>
      <p>Loading order details...</p>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="state-card glass-card error-card">
      <div class="error-icon">⚠️</div>
      <h3>Error Loading Order</h3>
      <p>{{ error.message }}</p>
      <button class="btn btn-secondary" @click="fetchOrder">Try Again</button>
    </div>

    <!-- Not Found -->
    <div v-else-if="!order" class="state-card glass-card">
      <div class="error-icon">🔍</div>
      <h2>Order Not Found</h2>
      <p>We couldn't find the requested order ID.</p>
      <router-link to="/orders" class="btn btn-primary"> View All Orders </router-link>
    </div>

    <!-- Order Detail Content -->
    <div v-else class="order-container">
      <!-- Order Header Card -->
      <div class="order-header-card glass-card">
        <div class="header-main">
          <div class="id-row">
            <span class="badge-label">ORDER</span>
            <span class="order-id">#{{ order.order_id }}</span>
          </div>
          <p class="order-date">Placed on {{ formatDate(order.created_at) }}</p>
        </div>
        <div class="status-pill" :class="getStatusClass(order.order_status)">
          <span class="status-dot"></span>
          {{ formatStatus(order.order_status) }}
        </div>
      </div>

      <!-- Main Layout Grid -->
      <div class="order-grid">
        <!-- Left: Items & Addresses -->
        <div class="order-main-col">
          <!-- Items Card -->
          <div class="section-card glass-card">
            <h2 class="card-heading">Purchased Items</h2>
            <div class="items-list">
              <div v-for="item in order.items" :key="item.product_id" class="order-item-row">
                <img
                  :src="getProductImage({ name: item.product_name })"
                  :alt="item.product_name"
                  class="item-thumb"
                />
                <div class="item-info">
                  <router-link :to="`/products/${item.product_id}`" class="item-title">
                    {{ item.product_name }}
                  </router-link>
                  <p v-if="item.product_brand" class="item-brand">
                    🏷️ {{ item.product_brand }}
                  </p>
                  <p class="item-calc">
                    {{ formatPrice(item.price, order.currency) }} ×
                    {{ item.quantity }}
                  </p>
                </div>
                <div class="item-total">
                  {{ formatPrice(item.price * item.quantity, order.currency) }}
                </div>
              </div>
            </div>

            <!-- Totals Divider -->
            <div class="divider"></div>

            <div class="summary-breakdown">
              <div class="breakdown-row">
                <span>Subtotal</span>
                <span class="val">{{ formatPrice(order.total, order.currency) }}</span>
              </div>
              <div class="breakdown-row">
                <span>Shipping</span>
                <span class="free-pill">FREE</span>
              </div>
              <div class="breakdown-row total-row">
                <span>Order Total</span>
                <span class="total-val">
                  {{ formatPrice(order.total, order.currency) }}
                </span>
              </div>
            </div>
          </div>

          <!-- Addresses Cards Grid -->
          <div class="addresses-grid">
            <!-- Shipping Card -->
            <div class="section-card glass-card">
              <h3 class="card-subheading">📍 Shipping Address</h3>
              <div v-if="order.shipping_address" class="address-body">
                <p v-if="order.shipping_address.name" class="name">
                  {{ order.shipping_address.name }}
                </p>
                <p>{{ order.shipping_address.street }}</p>
                <p>
                  {{ order.shipping_address.city }},
                  {{ order.shipping_address.state_or_region }}
                  {{ order.shipping_address.postal_code }}
                </p>
                <p>{{ order.shipping_address.country }}</p>
                <p v-if="order.shipping_address.phone_number" class="phone">
                  📞 {{ order.shipping_address.phone_number }}
                </p>
              </div>
              <p v-else class="text-muted">No shipping address recorded</p>
            </div>

            <!-- Billing Card -->
            <div class="section-card glass-card">
              <h3 class="card-subheading">💳 Billing & Contact</h3>
              <div class="address-body">
                <p v-if="order.email" class="email">✉️ {{ order.email }}</p>
                <div v-if="order.billing_address">
                  <p v-if="order.billing_address.name" class="name">
                    {{ order.billing_address.name }}
                  </p>
                  <p>{{ order.billing_address.street }}</p>
                  <p>
                    {{ order.billing_address.city }},
                    {{ order.billing_address.state_or_region }}
                    {{ order.billing_address.postal_code }}
                  </p>
                  <p>{{ order.billing_address.country }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Right: Status Timeline -->
        <div class="order-side-col">
          <div class="section-card glass-card timeline-card">
            <h2 class="card-heading">Fulfillment Timeline</h2>

            <div class="timeline">
              <div
                v-for="status in orderStatuses"
                :key="status.value"
                class="timeline-step"
                :class="{
                  active: isStatusActive(status.value),
                  completed: isStatusCompleted(status.value),
                }"
              >
                <div class="timeline-marker"></div>
                <div class="timeline-content">
                  <h4>{{ status.label }}</h4>
                  <p v-if="isStatusActive(status.value) && getStatusDate()">
                    Updated {{ formatDate(getStatusDate()!) }}
                  </p>
                  <p v-else-if="isStatusCompleted(status.value)">Completed</p>
                  <p v-else class="pending-label">Pending</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.order-detail-view {
  max-width: 1100px;
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

.order-container {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

/* Header Card */
.order-header-card {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.5rem 2rem;
}

.id-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.badge-label {
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: #c084fc;
  background: rgba(139, 92, 246, 0.15);
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  border: 1px solid rgba(139, 92, 246, 0.3);
}

.order-id {
  font-family: "JetBrains Mono", monospace;
  font-size: 1.3rem;
  font-weight: 700;
  color: var(--text-main);
}

.order-date {
  font-size: 0.85rem;
  color: var(--text-muted);
  margin-top: 0.4rem;
}

/* Status Pill */
.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.8rem;
  font-weight: 600;
  padding: 0.35rem 0.9rem;
  border-radius: 9999px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.status-processing {
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
  border: 1px solid rgba(245, 158, 11, 0.3);
}
.status-processing .status-dot {
  background: #fbbf24;
  box-shadow: 0 0 6px #fbbf24;
}

.status-shipped {
  background: rgba(6, 182, 212, 0.15);
  color: #38bdf8;
  border: 1px solid rgba(6, 182, 212, 0.3);
}
.status-shipped .status-dot {
  background: #38bdf8;
  box-shadow: 0 0 6px #38bdf8;
}

.status-delivered {
  background: rgba(52, 211, 153, 0.15);
  color: #34d399;
  border: 1px solid rgba(52, 211, 153, 0.3);
}
.status-delivered .status-dot {
  background: #34d399;
  box-shadow: 0 0 6px #34d399;
}

.status-cancelled {
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
  border: 1px solid rgba(239, 68, 68, 0.3);
}
.status-cancelled .status-dot {
  background: #f87171;
  box-shadow: 0 0 6px #f87171;
}

/* Grid */
.order-grid {
  display: grid;
  grid-template-columns: 1.6fr 1fr;
  gap: 1.5rem;
  align-items: start;
}

.order-main-col {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.section-card {
  padding: 1.5rem;
}

.card-heading {
  font-size: 1.15rem;
  font-weight: 700;
  margin: 0 0 1.25rem;
  color: var(--text-main);
}

.card-subheading {
  font-size: 1rem;
  font-weight: 600;
  margin: 0 0 1rem;
  color: var(--text-main);
}

/* Items List */
.items-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.order-item-row {
  display: grid;
  grid-template-columns: 60px 1fr auto;
  align-items: center;
  gap: 1rem;
}

.item-thumb {
  width: 60px;
  height: 60px;
  object-fit: cover;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: rgba(15, 23, 42, 0.6);
}

.item-info {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.item-title {
  color: var(--text-main);
  text-decoration: none;
  font-size: 0.95rem;
  font-weight: 500;
  transition: color 0.2s;
}

.item-title:hover {
  color: #c084fc;
}

.item-brand {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin: 0;
}

.item-calc {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.8rem;
  color: var(--text-muted);
  margin: 0;
}

.item-total {
  font-family: "JetBrains Mono", monospace;
  font-size: 1rem;
  font-weight: 600;
  color: var(--accent-cyan);
}

.divider {
  height: 1px;
  background: var(--border-color);
  margin: 1.5rem 0;
}

.summary-breakdown {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.breakdown-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.9rem;
  color: var(--text-muted);
}

.breakdown-row .val {
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

.total-row {
  padding-top: 0.75rem;
  border-top: 1px solid var(--border-color);
  margin-top: 0.4rem;
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--text-main);
}

.total-val {
  font-family: "JetBrains Mono", monospace;
  font-size: 1.4rem;
  font-weight: 700;
  color: var(--accent-cyan);
  text-shadow: 0 0 16px rgba(6, 182, 212, 0.3);
}

/* Addresses Grid */
.addresses-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.5rem;
}

.address-body {
  font-size: 0.9rem;
  color: var(--text-main);
  line-height: 1.6;
}

.address-body p {
  margin: 0.15rem 0;
}

.address-body .name {
  font-weight: 600;
}

.address-body .phone,
.address-body .email {
  color: var(--text-muted);
  font-size: 0.85rem;
  margin-top: 0.5rem;
}

/* Timeline */
.timeline-card {
  position: sticky;
  top: 90px;
}

.timeline {
  position: relative;
  padding-left: 1.5rem;
  margin: 1.5rem 0;
}

.timeline::before {
  content: "";
  position: absolute;
  left: 0.35rem;
  top: 6px;
  bottom: 6px;
  width: 2px;
  background: rgba(255, 255, 255, 0.1);
}

.timeline-step {
  position: relative;
  padding-bottom: 2rem;
  padding-left: 1.25rem;
}

.timeline-step:last-child {
  padding-bottom: 0;
}

.timeline-marker {
  position: absolute;
  left: -1.45rem;
  top: 0.2rem;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--bg-card);
  border: 2px solid var(--border-color);
  z-index: 1;
  transition: all 0.3s ease;
}

.timeline-step.completed .timeline-marker {
  background: var(--primary);
  border-color: #c084fc;
  box-shadow: 0 0 10px var(--primary-glow);
}

.timeline-step.active .timeline-marker {
  background: #38bdf8;
  border-color: #ffffff;
  box-shadow: 0 0 12px rgba(56, 189, 248, 0.6);
  animation: pulse-ring 2s infinite;
}

@keyframes pulse-ring {
  0% {
    box-shadow: 0 0 0 0 rgba(56, 189, 248, 0.6);
  }
  70% {
    box-shadow: 0 0 0 8px rgba(56, 189, 248, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(56, 189, 248, 0);
  }
}

.timeline-content h4 {
  margin: 0 0 0.2rem;
  font-size: 0.95rem;
  color: var(--text-main);
}

.timeline-content p {
  margin: 0;
  font-size: 0.8rem;
  color: var(--text-muted);
}

.pending-label {
  opacity: 0.5;
}

.agent-trace-note {
  display: flex;
  gap: 0.6rem;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  background: rgba(139, 92, 246, 0.08);
  border: 1px solid rgba(139, 92, 246, 0.2);
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 1.5rem;
}

.agent-trace-note code {
  color: #c084fc;
  font-family: "JetBrains Mono", monospace;
}

@media (max-width: 860px) {
  .order-grid {
    grid-template-columns: 1fr;
  }

  .addresses-grid {
    grid-template-columns: 1fr;
  }
}
</style>
