import { createApp } from "vue";
import { createPinia } from "pinia";
import {
  createRouter,
  createWebHistory,
  type RouteLocationNormalized,
} from "vue-router";
import App from "./App.vue";
import "./style.css";

// Import views
import HomeView from "./views/HomeView.vue";
import ProductListView from "./views/ProductListView.vue";
import ProductDetailView from "./views/ProductDetailView.vue";
import CartView from "./views/CartView.vue";
import CheckoutView from "./views/CheckoutView.vue";
import OrderListView from "./views/OrderListView.vue";
import OrderDetailView from "./views/OrderDetailView.vue";
import LoginView from "./views/LoginView.vue";
import NotFoundView from "./views/NotFoundView.vue";
import { useAuthStore } from "./stores/authStore";

// Initialize Pinia
const pinia = createPinia();

// Set up router
// Routes that require authentication
const requireAuth = (to: RouteLocationNormalized) => {
  const authStore = useAuthStore();
  if (!authStore.isAuthenticated) {
    return {
      path: "/login",
      query: { redirect: to.fullPath },
    };
  }
};

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: HomeView,
    },
    {
      path: "/login",
      name: "login",
      component: LoginView,
      meta: { requiresGuest: true },
    },
    {
      path: "/products",
      name: "products",
      component: ProductListView,
    },
    {
      path: "/products/:id",
      name: "product-detail",
      component: ProductDetailView,
      props: true,
    },
    {
      path: "/cart",
      name: "cart",
      component: CartView,
      beforeEnter: requireAuth,
    },
    {
      path: "/checkout",
      name: "checkout",
      component: CheckoutView,
      beforeEnter: requireAuth,
    },
    {
      path: "/orders",
      name: "orders",
      component: OrderListView,
      beforeEnter: requireAuth,
    },
    {
      path: "/orders/:id",
      name: "order-detail",
      component: OrderDetailView,
      props: true,
      beforeEnter: requireAuth,
    },
    {
      path: "/:pathMatch(.*)*",
      name: "not-found",
      component: NotFoundView,
    },
  ],
  scrollBehavior(to, from, savedPosition) {
    if (savedPosition) {
      return savedPosition;
    } else {
      return { top: 0 };
    }
  },
});

// Navigation guard to handle authentication
router.beforeEach((to, from, next) => {
  const authStore = useAuthStore();

  if (to.meta.requiresGuest && authStore.isAuthenticated) {
    // If user is authenticated and tries to access guest-only routes like login
    return next("/");
  }

  next();
});

// Create and mount the app
const app = createApp(App);

app.use(pinia);
app.use(router);

app.mount("#app");
