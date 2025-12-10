<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useOrderStore } from "@/stores/orderStore";
import { useAuthStore } from "@/stores/authStore";
import { getProductImage } from "@/api/services/productService";
import { formatPrice } from "@/utils/currency";

const orderStore = useOrderStore();
const authStore = useAuthStore();
const currentUserId = authStore.userId;

const orders = computed(() => orderStore.orders);
const isLoading = computed(() => orderStore.isLoading);
const error = computed(() => orderStore.error);

function formatDate(d: string) {
  return new Date(d).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

function formatStatus(status: string) {
  return status
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(" ");
}

function getStatusClass(status: string) {
  return {
    new: status === "PROCESSING",
    shipped: status === "SHIPPED",
    cancelled: status === "CANCELLED",
  };
}

async function fetchOrders() {
  await orderStore.fetchUserOrders(currentUserId);
}

onMounted(fetchOrders);
</script>

<template>
  <div class="order-list">
    <h1>Your Orders</h1>

    <div v-if="isLoading" class="loading">Loading your orders...</div>

    <div v-else-if="error" class="error">
      Error loading orders: {{ error.message }}
      <button class="btn btn-outline" @click="fetchOrders">Try Again</button>
    </div>

    <div v-else-if="!orders.length" class="no-orders">
      <p>You haven't placed any orders yet.</p>
      <router-link to="/products" class="btn"> Start Shopping </router-link>
    </div>

    <div v-else class="orders">
      <div v-for="order in orders" :key="order['order-id']" class="order-card">
        <div class="order-header">
          <div>
            <h3>Order #{{ order["order-id"] }}</h3>
            <p class="order-date">
              Placed on {{ formatDate(order["created-at"]) }}
            </p>
          </div>
          <div
            class="order-status"
            :class="getStatusClass(order['order-status'])"
          >
            {{ formatStatus(order["order-status"]) }}
          </div>
        </div>

        <div class="order-items">
          <div
            v-for="item in order.items.slice(0, 3)"
            :key="item['product-id']"
            class="order-item"
          >
            <img
              :src="getProductImage({ name: item['product-name'] })"
              :alt="item['product-name']"
              class="item-image"
            />
            <div class="item-details">
              <h4>
                <router-link
                  :to="`/products/${item['product-id']}`"
                  class="product-link"
                >
                  {{ item["product-name"] }}
                </router-link>
              </h4>
              <p>Qty: {{ item.quantity }}</p>
              <p class="price">{{ formatPrice(item.price, order.currency) }}</p>
            </div>
          </div>

          <div v-if="order.items.length > 3" class="more-items">
            +{{ order.items.length - 3 }} more item{{
              order.items.length - 3 > 1 ? "s" : ""
            }}
          </div>
        </div>

        <div class="order-footer">
          <div class="order-total">
            <p class="total-amount">
              Total: {{ formatPrice(order.total, order.currency) }}
            </p>
          </div>
          <router-link
            :to="`/orders/${order['order-id']}`"
            class="btn btn-outline"
          >
            View Order
          </router-link>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.order-list {
  max-width: 1000px;
  margin: 0 auto;
  padding: 2rem 1rem;
}

.loading,
.no-orders,
.error {
  text-align: center;
  padding: 4rem 1rem;
}

.error {
  color: #dc3545;
}

.orders {
  display: grid;
  gap: 1.5rem;
  margin-top: 2rem;
}

.order-card {
  background: white;
  border: 1px solid #e9ecef;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
}

.order-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.25rem;
  border-bottom: 1px solid #e9ecef;
  background: #f8f9fa;
}

.order-date {
  color: #6c757d;
  font-size: 0.875rem;
  margin-top: 0.25rem;
}

.order-status {
  padding: 0.35rem 0.75rem;
  border-radius: 50px;
  font-size: 0.8rem;
  font-weight: 500;
}

.status-processing {
  background: #fff3cd;
  color: #856404;
}

.status-shipped {
  background: #cce5ff;
  color: #004085;
}

.status-delivered {
  background: #d4edda;
  color: #155724;
}

.status-cancelled {
  background: #f8d7da;
  color: #721c24;
}

.order-items {
  padding: 1.25rem;
}

.order-item {
  display: flex;
  gap: 1rem;
  padding: 0.75rem 0;
  border-bottom: 1px solid #f1f3f5;
}

.order-item:last-child {
  border-bottom: none;
}

.item-image {
  width: 60px;
  height: 60px;
  object-fit: cover;
  border-radius: 4px;
  border: 1px solid #e9ecef;
}

.item-details {
  flex: 1;
}

.item-details h4 {
  margin: 0 0 0.25rem 0;
  font-size: 0.95rem;
}

.item-details p {
  margin: 0.1rem 0;
  font-size: 0.85rem;
  color: #6c757d;
}

.price {
  color: #2c3e50 !important;
  font-weight: 500;
}

.more-items {
  text-align: center;
  padding: 0.5rem;
  color: #6c757d;
  font-size: 0.9rem;
}

.order-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.25rem;
  border-top: 1px solid #e9ecef;
  background: #f8f9fa;
}

.order-total {
  font-size: 1.1rem;
  font-weight: 500;
}

.order-total span {
  color: #2c3e50;
  font-weight: 600;
}

.btn {
  padding: 0.5rem 1rem;
  border-radius: 4px;
  font-size: 0.9rem;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-outline {
  background: white;
  border: 1px solid #4a6fa5;
  color: #4a6fa5;
}

.btn-outline:hover {
  background: #f1f5f9;
}
</style>
