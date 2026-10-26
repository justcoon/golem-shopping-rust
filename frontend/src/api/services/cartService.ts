import apiClient from "../config";
import type { Address } from "@/types/address";
import { DateTime } from "@/types/datetime.ts";

export interface CartItem {
  "product_id": string;
  "product_name": string;
  "product_brand": string;
  price: number;
  quantity: number;
}

export interface Cart {
  "user_id": string;
  items: CartItem[];
  email?: string;
  "billing_address"?: Address;
  "shipping_address"?: Address;
  total: number;
  currency: string;
  "previous_order_ids": string[];
  "updated_at": DateTime;
}

export interface OrderConfirmation {
  "order_id": string;
}

export const getCart = async (userId: string): Promise<Cart> => {
  try {
    const response = await apiClient.get(`/v1/cart/${userId}`);
    return response;
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

const sanitizeAddress = (address: Address): Address => {
  const sanitized: Address = {
    street: address.street,
    city: address.city,
    state_or_region: address.state_or_region,
    country: address.country,
    postal_code: address.postal_code,
  };
  if (address.name) sanitized.name = address.name;
  if (address.phone_number) sanitized.phone_number = address.phone_number;
  return sanitized;
};

export const updateBillingAddress = async (
  userId: string,
  address: Address,
): Promise<Cart> => {
  try {
    const response = await apiClient.put(
      `/v1/cart/${userId}/billing-address`,
      sanitizeAddress(address),
    );
    return response;
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
      sanitizeAddress(address),
    );
    return response;
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
    return response;
  } catch (error) {
    console.error("Error during checkout:", error);
    throw error;
  }
};
