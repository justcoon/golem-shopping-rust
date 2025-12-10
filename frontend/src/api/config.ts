import axios, { AxiosInstance, AxiosResponse } from "axios";

const apiClient: AxiosInstance = axios.create({
  baseURL: "/api", // This will be proxied to http://golem-shopping.test.local
  headers: {
    "Content-Type": "application/json",
  },
});

// Request interceptor
apiClient.interceptors.request.use(
  (config) => {
    // You can add auth headers here if needed
    // const token = localStorage.getItem('auth_token');
    // if (token) {
    //   config.headers.Authorization = `Bearer ${token}`;
    // }
    return config;
  },
  (error) => {
    return Promise.reject(error);
  },
);

// Response interceptor
apiClient.interceptors.response.use(
  (response: AxiosResponse) => {
    return response.data;
  },
  (error) => {
    // Handle errors globally
    console.error("API Error:", error);
    return Promise.reject(error);
  },
);

export default apiClient;
