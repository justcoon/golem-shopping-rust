import apiClient from "../config";
import type { Address } from "@/types/address";

export interface CartItem {
  "product-id": string;
  "product-name": string;
  "product-brand": string;
  price: number;
  quantity: number;
}

export interface Cart {
  "user-id": string;
  items: CartItem[];
  email?: string;
  "billing-address"?: Address;
  "shipping-address"?: Address;
  total: number;
  currency: string;
  "previous-order-ids": string[];
  "updated-at": Date;
}

export interface OrderConfirmation {
  "order-id": string;
}

export const getCart = async (userId: string): Promise<Cart> => {
  try {
    const response = await apiClient.get(`/v1/cart/${userId}`);
    return response.ok;
  } catch (error) {
    console.error(`Error fetching cart for user ${userId}:`, error);
    throw error;
  }
};

export const addToCart = async (
  userId: string,
  productId: string,
  quantity: number = 1,
): Promise<void> => {
  try {
    await apiClient.put(`/v1/cart/${userId}/items/${productId}`, { quantity });
  } catch (error) {
    console.error(`Error adding item ${productId} to cart:`, error);
    throw error;
  }
};

export const removeFromCart = async (
  userId: string,
  productId: string,
): Promise<void> => {
  try {
    await apiClient.delete(`/v1/cart/${userId}/items/${productId}`);
  } catch (error) {
    console.error(`Error removing item ${productId} from cart:`, error);
    throw error;
  }
};

export const updateCartEmail = async (
  userId: string,
  email: string,
): Promise<void> => {
  try {
    await apiClient.put(`/v1/cart/${userId}/email`, { email });
  } catch (error) {
    console.error(`Error updating cart email:`, error);
    throw error;
  }
};

export const updateBillingAddress = async (
  userId: string,
  address: Address,
): Promise<Cart> => {
  try {
    const response = await apiClient.put(
      `/v1/cart/${userId}/billing-address`,
      address,
    );
    return response.ok;
  } catch (error) {
    console.error(`Error updating billing address for user ${userId}:`, error);
    throw error;
  }
};

export const updateShippingAddress = async (
  userId: string,
  address: Address,
): Promise<Cart> => {
  try {
    const response = await apiClient.put(
      `/v1/cart/${userId}/shipping-address`,
      address,
    );
    return response.ok;
  } catch (error) {
    console.error(`Error updating shipping address for user ${userId}:`, error);
    throw error;
  }
};

export const checkoutCart = async (
  userId: string,
): Promise<OrderConfirmation> => {
  try {
    const response = await apiClient.post(`/v1/cart/${userId}/checkout`, {});
    return response.ok;
  } catch (error) {
    console.error("Error during checkout:", error);
    throw error;
  }
};
