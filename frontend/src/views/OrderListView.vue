<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useOrderStore } from "@/stores/orderStore";
import { useAuthStore } from "@/stores/authStore";
import { getProductImage } from "@/api/services/productService";
import { DateTime, formatDate } from "@/types/datetime.ts";
import { formatPrice } from "@/utils/currency";

const orderStore = useOrderStore();
const authStore = useAuthStore();
const currentUserId = authStore.userId;

const orders = computed(() => orderStore.orders);
const isLoading = computed(() => orderStore.isLoading);
const error = computed(() => orderStore.error);

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

async function fetchOrders() {
  await orderStore.fetchUserOrders(currentUserId);
}

onMounted(fetchOrders);
</script>

<template>
  <div class="order-list-view">
    <div class="page-header">
      <div>
        <h1 class="gradient-text">Your Order History</h1>
        <p class="subtitle">Durable orders managed by Golem Order Agents</p>
      </div>
      <button class="btn btn-secondary refresh-btn" @click="fetchOrders">↻ Refresh</button>
    </div>

    <!-- Loading State -->
    <div v-if="isLoading" class="state-card glass-card">
      <div class="spinner"></div>
      <p>Loading your orders...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="state-card glass-card error-card">
      <div class="error-icon">⚠️</div>
      <h3>Error Loading Orders</h3>
      <p>{{ error.message }}</p>
      <button class="btn btn-secondary" @click="fetchOrders">Try Again</button>
    </div>

    <!-- Empty State -->
    <div v-else-if="!orders.length" class="state-card glass-card">
      <div class="empty-icon">📦</div>
      <h2>No orders placed yet</h2>
      <p>Your completed transactions will appear here.</p>
      <router-link to="/products" class="btn btn-primary"> Start Shopping </router-link>
    </div>

    <!-- Orders List -->
    <div v-else class="orders-grid">
      <div v-for="order in orders" :key="order.order_id" class="order-card glass-card">
        <div class="order-header">
          <div class="order-id-group">
            <span class="order-label">ORDER</span>
            <span class="order-id">#{{ order.order_id }}</span>
            <span class="order-date">{{ formatDate(order.created_at) }}</span>
          </div>
          <div class="status-pill" :class="getStatusClass(order.order_status)">
            <span class="status-dot"></span>
            {{ formatStatus(order.order_status) }}
          </div>
        </div>

        <div class="order-items-preview">
          <div v-for="item in order.items.slice(0, 3)" :key="item.product_id" class="item-row">
            <img
              :src="getProductImage({ name: item.product_name })"
              :alt="item.product_name"
              class="item-thumb"
            />
            <div class="item-info">
              <router-link :to="`/products/${item.product_id}`" class="item-title">
                {{ item.product_name }}
              </router-link>
              <div class="item-meta">
                <span>Qty: {{ item.quantity }}</span>
                <span class="item-price"> {{ formatPrice(item.price, order.currency) }} each </span>
              </div>
            </div>
            <div class="item-total">
              {{ formatPrice(item.price * item.quantity, order.currency) }}
            </div>
          </div>

          <div v-if="order.items.length > 3" class="more-items">
            +{{ order.items.length - 3 }} more item{{ order.items.length - 3 > 1 ? "s" : "" }}
          </div>
        </div>

        <div class="order-footer">
          <div class="order-total">
            <span class="total-label">Total Amount</span>
            <span class="total-val">
              {{ formatPrice(order.total, order.currency) }}
            </span>
          </div>
          <router-link :to="`/orders/${order.order_id}`" class="btn btn-secondary view-btn">
            View Details →
          </router-link>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.order-list-view {
  max-width: 1000px;
  margin: 0 auto;
  padding: 1.5rem 1rem 3rem;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 2rem;
}

.page-header h1 {
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 0.4rem;
}

.subtitle {
  color: var(--text-muted);
  font-size: 0.95rem;
}

.refresh-btn {
  padding: 0.5rem 1rem;
  font-size: 0.85rem;
}

.state-card {
  text-align: center;
  padding: 4rem 2rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
}

.empty-icon {
  font-size: 3.5rem;
}

.error-card {
  border-color: rgba(239, 68, 68, 0.3);
}

.error-icon {
  font-size: 3rem;
}

.orders-grid {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.order-card {
  overflow: hidden;
  display: flex;
  flex-direction: column;
  transition: all 0.2s ease;
}

.order-card:hover {
  border-color: rgba(255, 255, 255, 0.15);
}

.order-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.25rem 1.5rem;
  background: rgba(255, 255, 255, 0.02);
  border-bottom: 1px solid var(--border-color);
}

.order-id-group {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.order-label {
  font-size: 0.7rem;
  letter-spacing: 0.08em;
  font-weight: 700;
  color: var(--text-muted);
  background: rgba(255, 255, 255, 0.06);
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
}

.order-id {
  font-family: "JetBrains Mono", monospace;
  font-weight: 600;
  color: var(--text-main);
  font-size: 0.95rem;
}

.order-date {
  font-size: 0.85rem;
  color: var(--text-muted);
}

/* Status Pills */
.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.25rem 0.75rem;
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

/* Items Preview */
.order-items-preview {
  padding: 1.25rem 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.item-row {
  display: grid;
  grid-template-columns: 50px 1fr auto;
  align-items: center;
  gap: 1rem;
}

.item-thumb {
  width: 50px;
  height: 50px;
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

.item-meta {
  display: flex;
  gap: 0.75rem;
  font-size: 0.8rem;
  color: var(--text-muted);
}

.item-price {
  font-family: "JetBrains Mono", monospace;
}

.item-total {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--accent-cyan);
}

.more-items {
  text-align: center;
  padding: 0.4rem;
  font-size: 0.8rem;
  color: var(--text-muted);
  background: rgba(255, 255, 255, 0.02);
  border-radius: 6px;
}

/* Order Footer */
.order-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.25rem 1.5rem;
  border-top: 1px solid var(--border-color);
  background: rgba(255, 255, 255, 0.02);
}

.order-total {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
}

.total-label {
  font-size: 0.85rem;
  color: var(--text-muted);
}

.total-val {
  font-family: "JetBrains Mono", monospace;
  font-size: 1.3rem;
  font-weight: 700;
  color: var(--accent-cyan);
}

.view-btn {
  padding: 0.5rem 1.2rem;
  font-size: 0.85rem;
  font-weight: 600;
  text-decoration: none;
}
</style>
