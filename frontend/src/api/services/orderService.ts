import apiClient from "../config";
import type { Address } from "@/types/address";
import { DateTime } from "@/types/datetime.ts";

export interface OrderItem {
  product_id: string;
  product_name: string;
  product_brand: string;
  price: number;
  quantity: number;
}

export interface Order {
  order_id: string;
  user_id: string;
  items: OrderItem[];
  email?: string;
  billing_address?: Address;
  shipping_address?: Address;
  order_status: string;
  total: number;
  currency: string;
  created_at: DateTime;
  updated_at: DateTime;
}

export const getOrder = async (orderId: string): Promise<Order> => {
  try {
    return await apiClient.get(`/v1/order/${orderId}`);
  } catch (error) {
    console.error(`Error fetching order ${orderId}:`, error);
    throw error;
  }
};

export const getUserOrders = async (userId: string, orderIds: string[] = []): Promise<Order[]> => {
  try {
    if (!orderIds || orderIds.length === 0) {
      return [];
    }

    const orderPromises = orderIds.map((orderId) =>
      getOrder(orderId).catch((error) => {
        console.error(`Error fetching order ${orderId}:`, error);
        return null;
      }),
    );

    const orders = await Promise.all(orderPromises);
    return orders.filter((order): order is Order => order !== null);
  } catch (error) {
    console.error(`Error fetching orders for user ${userId}:`, error);
    throw error;
  }
};

export const updateOrderEmail = async (orderId: string, email: string): Promise<void> => {
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
): Promise<void> => {
  try {
    await apiClient.put(`/v1/order/${orderId}/billing-address`, address);
  } catch (error) {
    console.error(`Error updating billing address for order ${orderId}:`, error);
    throw error;
  }
};

export const updateOrderShippingAddress = async (
  orderId: string,
  address: Address,
): Promise<void> => {
  try {
    await apiClient.put(`/v1/order/${orderId}/shipping-address`, address);
  } catch (error) {
    console.error(`Error updating shipping address for order ${orderId}:`, error);
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
