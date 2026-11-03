<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "@/stores/authStore";

const authStore = useAuthStore();
const router = useRouter();
const userId = ref("");

const handleLogin = (idToUse?: string) => {
  const chosenId = idToUse || userId.value;
  if (!chosenId.trim()) return;
  authStore.login(chosenId.trim());
  const redirectPath = router.currentRoute.value.query.redirect?.toString() || "/";
  router.push(redirectPath);
};
</script>

<template>
  <div class="login-container">
    <div class="login-card glass-card">
      <div class="avatar-ring">
        <span class="avatar-icon">🛍️</span>
      </div>

      <div class="header-text">
        <h1 class="gradient-text">Welcome Back</h1>
        <p class="subtitle">Sign in to your autonomous shopping session</p>
      </div>

      <form class="login-form" @submit.prevent="handleLogin()">
        <div class="form-group">
          <label for="userId">User Identifier</label>
          <input
            id="userId"
            v-model="userId"
            type="text"
            required
            placeholder="e.g. user-alice or your-name"
            class="input-dark"
          />
        </div>

        <button type="submit" class="btn btn-primary submit-btn">Sign In →</button>
      </form>

      <div class="divider">
        <span>or try demo account</span>
      </div>

      <div class="quick-logins">
        <button type="button" class="demo-chip" @click="handleLogin('user-alice')">
          👤 user-alice
        </button>
        <button type="button" class="demo-chip" @click="handleLogin('user-bob')">
          👤 user-bob
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: calc(100vh - 220px);
  padding: 2rem 1rem;
}

.login-card {
  width: 100%;
  max-width: 420px;
  padding: 2.5rem 2rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
}

.avatar-ring {
  width: 68px;
  height: 68px;
  border-radius: 50%;
  background: rgba(139, 92, 246, 0.15);
  border: 1px solid rgba(139, 92, 246, 0.4);
  box-shadow: 0 0 20px rgba(139, 92, 246, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
}

.avatar-icon {
  font-size: 2rem;
}

.header-text {
  text-align: center;
}

.header-text h1 {
  font-size: 1.75rem;
  font-weight: 700;
  margin: 0 0 0.35rem;
}

.subtitle {
  color: var(--text-muted);
  font-size: 0.9rem;
  margin: 0;
}

.login-form {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.form-group label {
  font-size: 0.85rem;
  font-weight: 500;
  color: var(--text-muted);
}

.input-dark {
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  color: var(--text-main);
  font-size: 0.95rem;
  outline: none;
  transition: all 0.2s;
}

.input-dark:focus {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.2);
}

.submit-btn {
  width: 100%;
  padding: 0.85rem;
  font-size: 1rem;
  font-weight: 600;
  margin-top: 0.5rem;
}

.divider {
  width: 100%;
  display: flex;
  align-items: center;
  text-align: center;
  margin: 0.5rem 0 0;
}

.divider::before,
.divider::after {
  content: "";
  flex: 1;
  border-bottom: 1px solid var(--border-color);
}

.divider span {
  padding: 0 0.75rem;
  color: var(--text-muted);
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.quick-logins {
  display: flex;
  gap: 0.75rem;
  width: 100%;
  justify-content: center;
}

.demo-chip {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 0.5rem 1rem;
  color: var(--text-main);
  font-family: "JetBrains Mono", monospace;
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.2s;
}

.demo-chip:hover {
  background: rgba(139, 92, 246, 0.15);
  border-color: #c084fc;
  color: #c084fc;
}
</style>
