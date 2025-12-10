import { defineStore } from "pinia";
import { ref, computed } from "vue";
import {
  getCart,
  addToCart as addToCartApi,
  removeFromCart as removeFromCartApi,
  updateCartEmail as updateCartEmailApi,
  updateBillingAddress as updateBillingAddressApi,
  updateShippingAddress as updateShippingAddressApi,
  checkoutCart as checkoutCartApi,
  Cart,
} from "@/api/services/cartService";
import type { Address } from "@/types/address";

export const useCartStore = defineStore("cart", () => {
  const cart = ref<Cart | null>(null);
  const isLoading = ref(false);
  const error = ref<Error | null>(null);

  const cartItems = computed(() => cart.value?.items || []);
  const itemCount = computed(() =>
    cartItems.value.reduce((total, item) => total + item.quantity, 0),
  );

  const fetchCart = async (userId: string) => {
    isLoading.value = true;
    error.value = null;

    try {
      const cartData = await getCart(userId);
      cart.value = cartData;
      return cartData;
    } catch (err) {
      error.value = err as Error;
      console.error("Error fetching cart:", err);
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const addItem = async (
    userId: string,
    productId: string,
    quantity: number = 1,
  ) => {
    try {
      await addToCartApi(userId, productId, quantity);
      await fetchCart(userId);
    } catch (err) {
      error.value = err as Error;
      console.error("Error adding item to cart:", err);
      throw err;
    }
  };

  const removeItem = async (userId: string, productId: string) => {
    try {
      await removeFromCartApi(userId, productId);
      await fetchCart(userId);
    } catch (err) {
      error.value = err as Error;
      console.error("Error removing item from cart:", err);
      throw err;
    }
  };

  const updateItem = async (
    userId: string,
    productId: string,
    quantity: number,
  ) => {
    try {
      if (quantity <= 0) {
        // If quantity is 0 or negative, remove the item from the cart
        await removeItem(userId, productId);
        return;
      }

      // The addToCartApi handles both adding new items and updating quantities of existing items
      await addToCartApi(userId, productId, quantity);
      await fetchCart(userId);
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating item quantity in cart:", err);
      throw err;
    }
  };

  const updateEmail = async (userId: string, email: string) => {
    try {
      await updateCartEmailApi(userId, email);
      if (cart.value) {
        cart.value.email = email;
      }
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating cart email:", err);
      throw err;
    }
  };

  async function updateBillingAddress(userId: string, address: Address) {
    try {
      isLoading.value = true;
      error.value = null;
      const updatedCart = await updateBillingAddressApi(userId, address);
      cart.value = updatedCart;
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating billing address:", err);
      throw err;
    } finally {
      isLoading.value = false;
    }
  }

  async function updateShippingAddress(userId: string, address: Address) {
    try {
      isLoading.value = true;
      error.value = null;
      const updatedCart = await updateShippingAddressApi(userId, address);
      cart.value = updatedCart;
    } catch (err) {
      error.value = err as Error;
      console.error("Error updating shipping address:", err);
      throw err;
    }
  }

  const checkout = async (userId: string) => {
    try {
      const order = await checkoutCartApi(userId);
      cart.value = null; // Clear cart after successful checkout
      return order;
    } catch (err) {
      error.value = err as Error;
      console.error("Error during checkout:", err);
      throw err;
    }
  };

  const clearCart = () => {
    cart.value = null;
  };

  return {
    cart,
    cartItems,
    itemCount,
    isLoading,
    error,
    fetchCart,
    addItem,
    removeItem,
    updateItem,
    updateEmail,
    updateBillingAddress,
    updateShippingAddress,
    checkout,
    clearCart,
  };
});
