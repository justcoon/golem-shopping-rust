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
      if (!cartStore.cart) {
        await cartStore.fetchCart(userId);
      }

      const orderIds = cartStore.cart?.previous_order_ids || [];
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

      const orderIndex = orders.value.findIndex((o) => o.order_id === orderId);
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
      await updateOrderBillingAddressApi(orderId, address);
      const updatedOrder = await fetchOrder(orderId);

      const orderIndex = orders.value.findIndex((o) => o.order_id === orderId);
      if (orderIndex !== -1) {
        orders.value[orderIndex] = updatedOrder;
      }
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating order billing address:", err);
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const updateShippingAddress = async (orderId: string, address: Address) => {
    isLoading.value = true;
    error.value = null;

    try {
      await updateOrderShippingAddressApi(orderId, address);
      const updatedOrder = await fetchOrder(orderId);

      const orderIndex = orders.value.findIndex((o) => o.order_id === orderId);
      if (orderIndex !== -1) {
        orders.value[orderIndex] = updatedOrder;
      }
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating order shipping address:", err);
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const ship = async (orderId: string) => {
    try {
      await shipOrderApi(orderId);
      if (currentOrder.value) {
        currentOrder.value.order_status = "Shipped";
      }

      const orderIndex = orders.value.findIndex((o) => o.order_id === orderId);
      if (orderIndex !== -1) {
        orders.value[orderIndex].order_status = "Shipped";
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
        currentOrder.value.order_status = "Cancelled";
      }

      const orderIndex = orders.value.findIndex((o) => o.order_id === orderId);
      if (orderIndex !== -1) {
        orders.value[orderIndex].order_status = "Cancelled";
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
