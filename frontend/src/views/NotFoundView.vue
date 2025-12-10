<script setup lang="ts">
// No script logic needed for this component
</script>

<template>
  <div class="not-found">
    <div class="error-code">404</div>
    <h1>Page Not Found</h1>
    <p class="error-message">
      Oops! The page you're looking for doesn't exist or has been moved.
    </p>
    <div class="suggestions">
      <p>Here are some helpful links instead:</p>
      <ul>
        <li><router-link to="/">Home</router-link></li>
        <li><router-link to="/products">Products</router-link></li>
        <li><router-link to="/cart">Your Cart</router-link></li>
        <li><router-link to="/orders">Your Orders</router-link></li>
      </ul>
    </div>
    <router-link to="/" class="btn btn-primary"> Go to Homepage </router-link>
  </div>
</template>

<style scoped>
.not-found {
  max-width: 600px;
  margin: 0 auto;
  padding: 4rem 2rem;
  text-align: center;
}

.error-code {
  font-size: 8rem;
  font-weight: 800;
  color: #e9ecef;
  margin-bottom: -1.5rem;
  line-height: 1;
}

h1 {
  font-size: 2.5rem;
  margin-bottom: 1rem;
  color: #2c3e50;
}

.error-message {
  font-size: 1.25rem;
  color: #6c757d;
  margin-bottom: 2.5rem;
  line-height: 1.6;
}

.suggestions {
  background: #f8f9fa;
  border-radius: 8px;
  padding: 1.5rem;
  margin-bottom: 2rem;
  text-align: left;
}

.suggestions p {
  margin-top: 0;
  margin-bottom: 1rem;
  color: #495057;
  font-weight: 500;
}

.suggestions ul {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 0.75rem;
}

.suggestions a {
  display: block;
  padding: 0.5rem 1rem;
  color: #4a6fa5;
  text-decoration: none;
  border-radius: 4px;
  transition: all 0.2s;
  text-align: center;
  background: white;
  border: 1px solid #e9ecef;
}

.suggestions a:hover {
  background: #f1f5f9;
  color: #3a5a8c;
  border-color: #d1e0f0;
}

.btn {
  display: inline-block;
  padding: 0.75rem 2rem;
  background: #4a6fa5;
  color: white;
  text-decoration: none;
  border-radius: 4px;
  font-weight: 500;
  font-size: 1.1rem;
  transition: background-color 0.2s;
  border: none;
  cursor: pointer;
}

.btn:hover {
  background: #3a5a8c;
}

@media (max-width: 480px) {
  .error-code {
    font-size: 6rem;
  }

  h1 {
    font-size: 2rem;
  }

  .error-message {
    font-size: 1.1rem;
  }

  .suggestions ul {
    grid-template-columns: 1fr;
  }
}
</style>
