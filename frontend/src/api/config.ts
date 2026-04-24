import axios, { AxiosInstance, AxiosResponse } from "axios";

// Create a typed API client that returns the data directly
type ApiClient = AxiosInstance & {
  <T = any>(config: any): Promise<T>;
  get<T = any>(url: string, config?: any): Promise<T>;
  post<T = any>(url: string, data?: any, config?: any): Promise<T>;
  put<T = any>(url: string, data?: any, config?: any): Promise<T>;
  delete<T = any>(url: string, config?: any): Promise<T>;
};

const apiClient: ApiClient = axios.create({
  baseURL: "/api", // This will be proxied to http://golem-shopping.test.local
  headers: {
    "Content-Type": "application/json",
  },
}) as ApiClient;

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
