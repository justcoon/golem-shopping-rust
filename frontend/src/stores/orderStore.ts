import { defineStore } from "pinia";
import { ref } from "vue";
import {
  getOrder,
  getUserOrders as getUserOrdersApi,
  updateOrderEmail as updateOrderEmailApi,
  updateOrderBillingAddress as updateOrderBillingAddressApi,
  updateOrderShippingAddress as updateOrderShippingAddressApi,
  shipOrder as shipOrderApi,
  cancelOrder as cancelOrderApi,
  Order,
} from "@/api/services/orderService";
import { useCartStore } from "./cartStore";
import type { Address } from "@/types/address";

export const useOrderStore = defineStore("orders", () => {
  const orders = ref<Order[]>([]);
  const currentOrder = ref<Order | null>(null);
  const isLoading = ref(false);
  const error = ref<Error | null>(null);
  const cartStore = useCartStore();

  const fetchOrder = async (orderId: string) => {
    isLoading.value = true;
    error.value = null;

    try {
      const order = await getOrder(orderId);
      currentOrder.value = order;

      return order;
    } catch (err) {
      error.value = err as Error;
      console.error(`Error fetching order ${orderId}:`, err);
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const fetchUserOrders = async (userId: string) => {
    isLoading.value = true;
    error.value = null;

    try {
      // First, ensure we have the latest cart data
      if (!cartStore.cart) {
        await cartStore.fetchCart(userId);
      }

      // Get order IDs from the cart's previous-order-ids
      const orderIds = cartStore.cart?.["previous-order-ids"] || [];

      // Fetch orders using the order IDs from the cart
      const userOrders = await getUserOrdersApi(userId, orderIds);
      orders.value = userOrders;
      return userOrders;
    } catch (err) {
      error.value = err as Error;
      console.error(`Error fetching orders for user ${userId}:`, err);
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const updateEmail = async (orderId: string, email: string) => {
    try {
      await updateOrderEmailApi(orderId, email);
      if (currentOrder.value) {
        currentOrder.value.email = email;
      }

      // Also update in the orders list if it exists there
      const orderIndex = orders.value.findIndex(
        (o) => o["order-id"] === orderId,
      );
      if (orderIndex !== -1) {
        orders.value[orderIndex].email = email;
      }
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating order email:", err);
      throw err;
    }
  };

  const updateBillingAddress = async (orderId: string, address: Address) => {
    isLoading.value = true;
    error.value = null;

    try {
      const updatedOrder = await updateOrderBillingAddressApi(orderId, address);

      // Update local state
      if (currentOrder.value && currentOrder.value["order-id"] === orderId) {
        currentOrder.value = updatedOrder;
      }

      // Also update in the orders list if it exists there
      const orderIndex = orders.value.findIndex(
        (o) => o["order-id"] === orderId,
      );
      if (orderIndex !== -1) {
        orders.value[orderIndex]["billing-address"] = address;
      }
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating order billing address:", err);
      throw err;
    }
  };

  const updateShippingAddress = async (orderId: string, address: Address) => {
    isLoading.value = true;
    error.value = null;

    try {
      const updatedOrder = await updateOrderShippingAddressApi(
        orderId,
        address,
      );

      // Update local state
      if (currentOrder.value && currentOrder.value["order-id"] === orderId) {
        currentOrder.value = updatedOrder;
      }

      // Also update in the orders list if it exists there
      const orderIndex = orders.value.findIndex(
        (o) => o["order-id"] === orderId,
      );
      if (orderIndex !== -1) {
        orders.value[orderIndex]["shipping-address"] = address;
      }
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating order shipping address:", err);
      throw err;
    }
  };

  const ship = async (orderId: string) => {
    try {
      await shipOrderApi(orderId);
      if (currentOrder.value) {
        currentOrder.value["order-status"] = "shipped";
      }

      // Update status in the orders list if it exists there
      const orderIndex = orders.value.findIndex(
        (o) => o["order-id"] === orderId,
      );
      if (orderIndex !== -1) {
        orders.value[orderIndex]["order-status"] = "shipped";
      }
    } catch (err) {
      error.value = err as Error;
      console.error(`Error shipping order ${orderId}:`, err);
      throw err;
    }
  };

  const cancel = async (orderId: string) => {
    try {
      await cancelOrderApi(orderId);
      if (currentOrder.value) {
        currentOrder.value["order-status"] = "cancelled";
      }

      // Update status in the orders list if it exists there
      const orderIndex = orders.value.findIndex(
        (o) => o["order-id"] === orderId,
      );
      if (orderIndex !== -1) {
        orders.value[orderIndex]["order-status"] = "cancelled";
      }
    } catch (err) {
      error.value = err as Error;
      console.error(`Error cancelling order ${orderId}:`, err);
      throw err;
    }
  };

  const clearCurrentOrder = () => {
    currentOrder.value = null;
  };

  return {
    orders,
    currentOrder,
    isLoading,
    error,
    fetchOrder,
    fetchUserOrders,
    updateEmail,
    updateBillingAddress,
    updateShippingAddress,
    ship,
    cancel,
    clearCurrentOrder,
  };
});
