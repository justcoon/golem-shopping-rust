import apiClient from "../config";
import type { Address } from "@/types/address";

export interface OrderItem {
  "product-id": string;
  "product-name": string;
  "product-brand": string;
  price: number;
  quantity: number;
}

export interface Order {
  "order-id": string;
  "user-id": string;
  items: OrderItem[];
  email: string;
  "billing-address"?: Address;
  "shipping-address"?: Address;
  "order-status": string;
  total: number;
  currency: string;
  "created-at": string;
  "updated-at": string;
}

export const getOrder = async (orderId: string): Promise<Order> => {
  try {
    const response = await apiClient.get(`/v1/order/${orderId}`);
    return response.ok;
  } catch (error) {
    console.error(`Error fetching order ${orderId}:`, error);
    throw error;
  }
};

export const getUserOrders = async (
  userId: string,
  orderIds: string[] = [],
): Promise<Order[]> => {
  try {
    if (!orderIds || orderIds.length === 0) {
      return [];
    }

    // Fetch all orders in parallel
    const orderPromises = orderIds.map((orderId) =>
      getOrder(orderId).catch((error) => {
        console.error(`Error fetching order ${orderId}:`, error);
        return null;
      }),
    );

    const orders = await Promise.all(orderPromises);

    // Filter out any failed order fetches and ensure we only return valid orders
    return orders.filter((order): order is Order => order !== null);
  } catch (error) {
    console.error(`Error fetching orders for user ${userId}:`, error);
    throw error;
  }
};

export const updateOrderEmail = async (
  orderId: string,
  email: string,
): Promise<void> => {
  try {
    await apiClient.put(`/v1/order/${orderId}/email`, { email });
  } catch (error) {
    console.error(`Error updating order email:`, error);
    throw error;
  }
};

export const updateOrderBillingAddress = async (
  orderId: string,
  address: Address,
): Promise<Order> => {
  try {
    const response = await apiClient.put(
      `/v1/order/${orderId}/billing-address`,
      address,
    );
    return response.data;
  } catch (error) {
    console.error(
      `Error updating billing address for order ${orderId}:`,
      error,
    );
    throw error;
  }
};

export const updateOrderShippingAddress = async (
  orderId: string,
  address: Address,
): Promise<Order> => {
  try {
    const response = await apiClient.put(
      `/v1/order/${orderId}/shipping-address`,
      address,
    );
    return response.data;
  } catch (error) {
    console.error(
      `Error updating shipping address for order ${orderId}:`,
      error,
    );
    throw error;
  }
};

export const shipOrder = async (orderId: string): Promise<void> => {
  try {
    await apiClient.post(`/v1/order/${orderId}/ship-order`, {});
  } catch (error) {
    console.error(`Error shipping order ${orderId}:`, error);
    throw error;
  }
};

export const cancelOrder = async (orderId: string): Promise<void> => {
  try {
    await apiClient.post(`/v1/order/${orderId}/cancel-order`, {});
  } catch (error) {
    console.error(`Error cancelling order ${orderId}:`, error);
    throw error;
  }
};
