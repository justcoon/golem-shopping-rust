<script setup lang="ts">
import { computed, watch, onMounted } from "vue";
import { useRoute } from "vue-router";
import { useOrderStore } from "@/stores/orderStore";
import { getProductImage } from "@/api/services/productService";
import { formatPrice } from "@/utils/currency";

const route = useRoute();
const orderStore = useOrderStore();

const order = computed(() => orderStore.currentOrder);
const isLoading = computed(() => orderStore.isLoading);
const error = computed(() => orderStore.error);

const orderStatuses = [
  { value: "PROCESSING", label: "Order Placed" },
  // { value: 'SHIPPED', label: 'Shipped' },
  // { value: 'DELIVERED', label: 'Delivered' }
];

function formatDate(d: string) {
  return new Date(d).toLocaleDateString("en-US", {
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
  return {
    new: status === "PROCESSING",
    shipped: status === "SHIPPED",
    cancelled: status === "CANCELLED",
  };
}

function isStatusActive(status: string) {
  if (!order.value) return false;
  return order.value["order-status"] === status;
}

function isStatusCompleted(status: string) {
  if (!order.value) return false;
  const statusOrder = ["PROCESSING", "SHIPPED", "DELIVERED"];
  const currentStatusIndex = statusOrder.indexOf(order.value["order-status"]);
  const statusIndex = statusOrder.indexOf(status);
  return currentStatusIndex >= statusIndex;
}

function getStatusDate() {
  if (!order.value) return null;
  return order.value["updated-at"];
}

async function fetchOrder() {
  const orderId = route.params.id as string;
  if (orderId) {
    await orderStore.fetchOrder(orderId);
  }
}

onMounted(fetchOrder);

// Watch for route changes to load the correct order
watch(() => route.params.id, fetchOrder);
</script>

<template>
  <div class="order-detail">
    <div v-if="isLoading" class="loading">Loading order details...</div>

    <div v-else-if="error" class="error">
      Error loading order: {{ error.message }}
      <button class="btn btn-outline" @click="fetchOrder">Try Again</button>
    </div>

    <div v-else-if="!order" class="not-found">
      <h2>Order not found</h2>
      <p>We couldn't find the order you're looking for.</p>
      <router-link to="/orders" class="btn"> View All Orders </router-link>
    </div>

    <div v-else class="order-container">
      <div class="order-header">
        <div>
          <h1>Order #{{ order["order-id"] }}</h1>
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

      <div class="order-grid">
        <div class="order-items">
          <h2>Order Items</h2>
          <div
            v-for="item in order.items"
            :key="item['product-id']"
            class="order-item"
          >
            <img
              :src="getProductImage({ name: item['product-name'] })"
              :alt="item['product-name']"
              class="item-image"
            />
            <div class="item-details">
              <h3>
                <router-link
                  :to="`/products/${item['product-id']}`"
                  class="product-link"
                >
                  {{ item["product-name"] }}
                </router-link>
              </h3>
              <p class="item-brand">{{ item["product-brand"] }}</p>
              <p class="item-price">
                {{ formatPrice(item.price, order.currency) }} ×
                {{ item.quantity }}
              </p>
            </div>
            <div class="item-total">
              {{ formatPrice(item.price * item.quantity, order.currency) }}
            </div>
          </div>

          <h2>Order Summary</h2>

          <!-- Order Summary -->
          <div class="address-section">
            <h3>Contact Information</h3>
            <div v-if="order.email" class="address-details">
              <p><strong>Email:</strong> {{ order.email }}</p>
            </div>
          </div>

          <!-- Shipping Address -->
          <div class="address-section">
            <h3>Shipping Address</h3>
            <div v-if="order['shipping-address']" class="address-details">
              <p v-if="order['shipping-address'].name">
                <strong>{{ order["shipping-address"].name }}</strong>
              </p>
              <p>{{ order["shipping-address"].street }}</p>
              <p>
                {{ order["shipping-address"].city }},
                {{ order["shipping-address"]["state-or-region"] }}
                {{ order["shipping-address"]["postal-code"] }}
              </p>
              <p>{{ order["shipping-address"].country }}</p>
              <p v-if="order['shipping-address']['phone-number']">
                <i class="fas fa-phone"></i>
                {{ order["shipping-address"]["phone-number"] }}
              </p>
            </div>
            <p v-else>No shipping address provided</p>
          </div>

          <!-- Billing Address -->
          <div v-if="order['billing-address']" class="address-section">
            <h3>Billing Address</h3>
            <div class="address-details">
              <p v-if="order['billing-address'].name">
                <strong>{{ order["billing-address"].name }}</strong>
              </p>
              <p>{{ order["billing-address"].street }}</p>
              <p>
                {{ order["billing-address"].city }},
                {{ order["billing-address"]["state-or-region"] }}
                {{ order["billing-address"]["postal-code"] }}
              </p>
              <p>{{ order["billing-address"].country }}</p>
              <p v-if="order['billing-address']['phone-number']">
                <i class="fas fa-phone"></i>
                {{ order["billing-address"]["phone-number"] }}
              </p>
            </div>
          </div>

          <div class="order-summary">
            <div class="summary-row total">
              <span>Total</span>
              <p class="total">
                {{ formatPrice(order.total, order.currency) }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <div class="order-timeline">
        <h2>Order Status</h2>
        <div class="timeline">
          <div
            v-for="(status, index) in orderStatuses"
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
              <p v-if="getStatusDate()">
                {{ formatDate(getStatusDate()!) }}
              </p>
              <p v-else>Pending</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.address-section {
  background: #f8f9fa;
  border-radius: 8px;
  padding: 1.25rem;
  margin-bottom: 1.5rem;
  border: 1px solid #e9ecef;
}

.address-section h3 {
  margin-top: 0;
  margin-bottom: 1rem;
  color: #343a40;
  font-size: 1.1rem;
  font-weight: 600;
}

.address-details p {
  margin: 0.25rem 0;
  color: #495057;
  line-height: 1.5;
}

.address-details p:last-child {
  margin-bottom: 0;
}

.address-details i {
  margin-right: 0.5rem;
  color: #6c757d;
  width: 1rem;
  text-align: center;
}

.order-detail {
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem 1rem;
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

.order-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 2rem;
  padding-bottom: 1.5rem;
  border-bottom: 1px solid #e9ecef;
}

.order-date {
  color: #6c757d;
  font-size: 1rem;
  margin-top: 0.5rem;
}

.order-status {
  padding: 0.5rem 1rem;
  border-radius: 50px;
  font-size: 0.9rem;
  font-weight: 500;
  text-transform: capitalize;
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

.order-grid {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 2rem;
  margin-bottom: 3rem;
}

.order-items {
  background: white;
  border: 1px solid #e9ecef;
  border-radius: 8px;
  padding: 1.5rem;
}

.order-item {
  display: flex;
  gap: 1.5rem;
  padding: 1.5rem 0;
  border-bottom: 1px solid #f1f3f5;
}

.order-item:last-child {
  border-bottom: none;
}

.item-image {
  width: 80px;
  height: 80px;
  object-fit: cover;
  border-radius: 4px;
  border: 1px solid #e9ecef;
}

.item-details {
  flex: 1;
}

.item-details h3 {
  margin: 0 0 0.5rem 0;
  font-size: 1.1rem;
}

.item-brand {
  color: #6c757d;
  font-size: 0.9rem;
  margin: 0 0 0.5rem 0;
}

.item-price {
  color: #495057;
  font-size: 0.95rem;
  margin: 0;
}

.item-total {
  font-weight: 500;
  font-size: 1.1rem;
}

.order-summary {
  margin-top: 2rem;
  padding-top: 1.5rem;
  border-top: 1px solid #e9ecef;
}

.summary-row {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.75rem;
  font-size: 0.95rem;
}

.summary-row.total {
  font-size: 1.1rem;
  font-weight: 600;
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid #e9ecef;
}

.order-info > div {
  background: white;
  border: 1px solid #e9ecef;
  border-radius: 8px;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
}

.order-info h3 {
  margin-top: 0;
  margin-bottom: 1rem;
  font-size: 1.1rem;
  color: #343a40;
}

.order-info p {
  margin: 0.5rem 0;
  color: #495057;
  line-height: 1.5;
}

.order-actions {
  text-align: right;
  padding-top: 1rem;
  border-top: 1px solid #e9ecef;
  margin-top: 1.5rem;
}

.order-timeline {
  background: white;
  border: 1px solid #e9ecef;
  border-radius: 8px;
  padding: 1.5rem;
  margin-top: 2rem;
}

.timeline {
  position: relative;
  padding-left: 2rem;
  margin-top: 1.5rem;
}

.timeline::before {
  content: "";
  position: absolute;
  left: 0.5rem;
  top: 0;
  bottom: 0;
  width: 2px;
  background: #e9ecef;
}

.timeline-step {
  position: relative;
  padding-bottom: 2rem;
  padding-left: 2rem;
}

.timeline-step:last-child {
  padding-bottom: 0;
}

.timeline-marker {
  position: absolute;
  left: -1.5rem;
  top: 0.25rem;
  width: 1rem;
  height: 1rem;
  border-radius: 50%;
  background: #e9ecef;
  border: 3px solid white;
  z-index: 1;
}

.timeline-step.completed .timeline-marker {
  background: #4a6fa5;
  border-color: white;
}

.timeline-step.active .timeline-marker {
  background: white;
  border: 3px solid #4a6fa5;
}

.timeline-content h4 {
  margin: 0 0 0.25rem 0;
  font-size: 1rem;
  color: #343a40;
}

.timeline-content p {
  margin: 0;
  color: #6c757d;
  font-size: 0.9rem;
}

.btn {
  display: inline-block;
  padding: 0.5rem 1.25rem;
  border-radius: 4px;
  font-size: 0.95rem;
  font-weight: 500;
  text-align: center;
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
  background: #f8f9fa;
}

@media (max-width: 768px) {
  .order-grid {
    grid-template-columns: 1fr;
  }

  .order-header {
    flex-direction: column;
    gap: 1rem;
  }

  .order-status {
    align-self: flex-start;
  }
}
</style>
