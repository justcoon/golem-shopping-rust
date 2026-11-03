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
  <div class="app-layout">
    <!-- Modern Sticky Glassmorphism Header -->
    <header class="navbar-header glass">
      <div class="header-container">
        <!-- Brand / Logo -->
        <router-link to="/" class="brand">
          <span class="brand-icon">🛍️</span>
          <div class="brand-info">
            <h1 class="gradient-text">Golem Shopping</h1>
            <span class="tagline">Agentic Autonomous E-Commerce</span>
          </div>
        </router-link>

        <!-- Navigation Tabs -->
        <nav class="nav-links">
          <router-link to="/" class="tab-link" exact-active-class="active">
            <span class="tab-icon">🏠</span>
            <span>Home</span>
          </router-link>

          <router-link to="/products" class="tab-link" active-class="active">
            <span class="tab-icon">📦</span>
            <span>Catalog</span>
          </router-link>

          <router-link
            v-if="authStore.isAuthenticated"
            to="/cart"
            class="tab-link cart-link"
            active-class="active"
          >
            <span class="tab-icon">🛒</span>
            <span>Cart</span>
            <span v-if="cartItemCount > 0" class="cart-badge">{{ cartItemCount }}</span>
          </router-link>

          <router-link
            v-if="authStore.isAuthenticated"
            to="/orders"
            class="tab-link"
            active-class="active"
          >
            <span class="tab-icon">📋</span>
            <span>My Orders</span>
          </router-link>

          <router-link
            v-if="authStore.isAuthenticated"
            to="/assistant"
            class="tab-link assistant-tab-link"
            active-class="active"
          >
            <span class="tab-icon">🤖</span>
            <span>AI Assistant</span>
          </router-link>
        </nav>

        <!-- User / Auth -->
        <div class="header-auth">
          <div v-if="authStore.isAuthenticated" class="user-pill">
            <span class="user-avatar">👤</span>
            <span class="user-id">{{ authStore.userId }}</span>
            <button class="logout-btn" @click="handleLogout" title="Logout">✕</button>
          </div>
          <router-link v-else to="/login" class="btn btn-primary btn-sm"> Login </router-link>
        </div>
      </div>
    </header>

    <!-- Main View Content Area -->
    <main class="main-content">
      <router-view v-slot="{ Component }">
        <transition name="fade" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </main>

    <!-- Footer -->
    <footer class="app-footer">
      <div class="footer-container">
        <p>
          Built with <strong>Golem Cloud</strong>, <strong>Rust</strong> &amp;
          <strong>Vue 3</strong>
        </p>
      </div>
    </footer>

    <!-- Floating AI Assistant Launcher -->
    <router-link
      v-if="authStore.isAuthenticated && router.currentRoute.value.path !== '/assistant'"
      to="/assistant"
      class="floating-assistant-fab"
      title="Open AI Shopping Assistant"
    >
      <span class="fab-icon">🤖</span>
      <span class="fab-text">Assistant</span>
    </router-link>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}

.navbar-header {
  position: sticky;
  top: 0;
  z-index: 1000;
  border-radius: 0 0 20px 20px;
  border-top: none;
  border-left: none;
  border-right: none;
  background: rgba(11, 15, 25, 0.88);
  box-shadow: 0 4px 30px rgba(0, 0, 0, 0.45);
}

.header-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 14px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  text-decoration: none;
}

.brand-icon {
  font-size: 1.8rem;
  background: rgba(139, 92, 246, 0.15);
  border: 1px solid rgba(139, 92, 246, 0.35);
  padding: 6px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.brand-info h1 {
  font-size: 1.4rem;
  margin: 0;
  line-height: 1.1;
}

.tagline {
  font-size: 0.75rem;
  color: var(--text-muted);
  letter-spacing: 0.04em;
}

.nav-links {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.tab-link {
  background: transparent;
  border: 1px solid transparent;
  color: var(--text-muted);
  padding: 7px 14px;
  border-radius: 8px;
  font-size: 0.9rem;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 6px;
  text-decoration: none;
  transition: all 0.2s;
}

.tab-link:hover {
  color: #ffffff;
  background: rgba(255, 255, 255, 0.06);
}

.tab-link.active {
  background: rgba(139, 92, 246, 0.2);
  color: #c084fc;
  border: 1px solid rgba(139, 92, 246, 0.35);
}

.assistant-tab-link {
  background: rgba(139, 92, 246, 0.1);
  border-color: rgba(139, 92, 246, 0.2);
  color: #d8b4fe;
}

.assistant-tab-link:hover {
  background: rgba(139, 92, 246, 0.25);
  color: #ffffff;
}

.cart-link {
  position: relative;
}

.cart-badge {
  background: linear-gradient(135deg, #ec4899 0%, #ef4444 100%);
  color: white;
  border-radius: 9999px;
  font-size: 0.72rem;
  font-weight: 700;
  padding: 1px 7px;
  min-width: 18px;
  text-align: center;
  box-shadow: 0 0 10px rgba(236, 72, 153, 0.5);
}

.header-auth {
  display: flex;
  align-items: center;
  gap: 10px;
}

.user-pill {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border-color);
  padding: 5px 12px;
  border-radius: 20px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.85rem;
}

.user-id {
  color: #38bdf8;
  font-family: var(--font-mono);
  font-weight: 600;
}

.logout-btn {
  background: transparent;
  border: none;
  color: var(--text-dim);
  font-size: 0.85rem;
  padding: 2px 6px;
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.2s;
}

.logout-btn:hover {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.15);
}

.main-content {
  flex: 1;
  width: 100%;
  padding: 24px 0 40px;
}

.app-footer {
  border-top: 1px solid var(--border-color);
  padding: 20px 0;
  text-align: center;
  font-size: 0.85rem;
  color: var(--text-dim);
  background: rgba(11, 15, 25, 0.6);
}

.app-footer strong {
  color: var(--text-muted);
}

/* Floating AI Assistant FAB */
.floating-assistant-fab {
  position: fixed;
  bottom: 24px;
  right: 24px;
  background: linear-gradient(135deg, #8b5cf6 0%, #6366f1 100%);
  color: white;
  border-radius: 28px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 20px;
  font-size: 14px;
  font-weight: 600;
  box-shadow: 0 6px 25px rgba(139, 92, 246, 0.45);
  cursor: pointer;
  z-index: 999;
  text-decoration: none;
  border: 1px solid rgba(255, 255, 255, 0.2);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.floating-assistant-fab:hover {
  transform: translateY(-3px) scale(1.03);
  box-shadow: 0 10px 30px rgba(139, 92, 246, 0.6);
}

.fab-icon {
  font-size: 20px;
}

/* Page transitions */
.fade-enter-active,
.fade-leave-active {
  transition:
    opacity 0.15s ease,
    transform 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
