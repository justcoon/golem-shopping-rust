<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useCartStore } from "./stores/cartStore";
import { useAuthStore } from "@/stores/authStore";

const router = useRouter();
const cartStore = useCartStore();
const authStore = useAuthStore();

const cartItemCount = computed(() => cartStore.itemCount);

const handleLogout = () => {
  authStore.logout();
  router.push("/");
};

// Fetch cart when the app loads if user is authenticated
onMounted(() => {
  if (authStore.userId) {
    cartStore.fetchCart(authStore.userId);
  }
});
</script>

<template>
  <div class="app">
    <header class="header">
      <nav class="nav">
        <router-link to="/" class="logo">Golem Shopping</router-link>
        <div class="nav-links">
          <router-link to="/">Home</router-link>
          <router-link to="/products">Products</router-link>
          <router-link
            v-if="authStore.isAuthenticated"
            to="/cart"
            class="cart-link"
          >
            Cart
            <span v-if="cartItemCount > 0" class="cart-count">{{
              cartItemCount
            }}</span>
          </router-link>
          <router-link v-if="authStore.isAuthenticated" to="/orders"
            >My Orders</router-link
          >
          <div v-if="authStore.isAuthenticated" class="user-info">
            <span>User: {{ authStore.userId }} </span>
            <button class="logout-btn" @click="handleLogout">Logout</button>
          </div>
          <router-link v-else to="/login" class="login-btn">Login</router-link>
        </div>
      </nav>
    </header>

    <main class="main-content">
      <router-view v-slot="{ Component }">
        <transition name="fade" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </main>

    <footer class="footer">
      <div class="container">
        <p>&copy; 2025 Golem Shopping. All rights reserved.</p>
      </div>
    </footer>
  </div>
</template>

<style>
:root {
  --primary-color: #4a6fa5;
  --secondary-color: #6c757d;
  --success-color: #28a745;
  --danger-color: #dc3545;
  --light-color: #f8f9fa;
  --dark-color: #343a40;
  --border-radius: 4px;
  --box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family:
    -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue",
    Arial, sans-serif;
  line-height: 1.6;
  color: #333;
  background-color: #f5f5f5;
}

.app {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}

.header {
  background-color: var(--primary-color);
  color: white;
  padding: 1rem 0;
  box-shadow: var(--box-shadow);
}

.nav {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.logo {
  font-size: 1.5rem;
  font-weight: bold;
  color: white;
  text-decoration: none;
}

.nav-links {
  display: flex;
  gap: 1.5rem;
  align-items: center;
}

.nav-links a {
  color: white;
  text-decoration: none;
  padding: 0.5rem 0;
  position: relative;
  transition: opacity 0.2s;
}

.nav-links a:hover {
  opacity: 0.9;
}

.nav-links a.router-link-active {
  font-weight: 500;
}

.nav-links a.router-link-active::after {
  content: "";
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  height: 2px;
  background-color: white;
}

.cart-link {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.cart-count {
  background-color: var(--danger-color);
  color: white;
  border-radius: 50%;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: bold;
}

.main-content {
  flex: 1;
  padding: 2rem 0;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1rem;
}

.footer {
  background-color: var(--dark-color);
  color: white;
  padding: 1.5rem 0;
  text-align: center;
  margin-top: auto;
}

/* Transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* Buttons */
.btn {
  display: inline-block;
  font-weight: 400;
  text-align: center;
  white-space: nowrap;
  vertical-align: middle;
  user-select: none;
  border: 1px solid transparent;
  padding: 0.375rem 0.75rem;
  font-size: 1rem;
  line-height: 1.5;
  border-radius: var(--border-radius);
  transition:
    color 0.15s ease-in-out,
    background-color 0.15s ease-in-out,
    border-color 0.15s ease-in-out,
    box-shadow 0.15s ease-in-out;
  cursor: pointer;
  text-decoration: none;
}

.btn-primary {
  color: #fff;
  background-color: var(--primary-color);
  border-color: var(--primary-color);
}

.btn-primary:hover {
  background-color: #3a5a8c;
  border-color: #365486;
}

.btn-outline-primary {
  color: var(--primary-color);
  border-color: var(--primary-color);
  background-color: transparent;
}

.btn-outline-primary:hover {
  color: #fff;
  background-color: var(--primary-color);
  border-color: var(--primary-color);
}

/* Utility classes */
.text-center {
  text-align: center;
}

.mt-1 {
  margin-top: 0.5rem;
}
.mt-2 {
  margin-top: 1rem;
}
.mt-3 {
  margin-top: 1.5rem;
}
.mt-4 {
  margin-top: 2rem;
}

.mb-1 {
  margin-bottom: 0.5rem;
}
.mb-2 {
  margin-bottom: 1rem;
}
.mb-3 {
  margin-bottom: 1.5rem;
}
.mb-4 {
  margin-bottom: 2rem;
}

.flex {
  display: flex;
}

.items-center {
  align-items: center;
}

.justify-between {
  justify-content: space-between;
}

.gap-1 {
  gap: 0.5rem;
}
.gap-2 {
  gap: 1rem;
}
.gap-3 {
  gap: 1.5rem;
}
.gap-4 {
  gap: 2rem;
}
</style>
