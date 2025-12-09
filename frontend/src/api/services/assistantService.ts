import apiClient from "../config";
import type {Product} from "./productService.ts";
import {getBatchPricing, getBestPrice, PriceFilterOptions} from "@/api/services/pricingService.ts";

export const getRecommendedProducts = async (userId: string, options?: PriceFilterOptions): Promise<Product[]> => {
    try {
        const products: Product[] = await apiClient.get(`/v1/assistant/${userId}/recommended-products`);

        // Get pricing for all products in batch
        const productIds = products.map((p) => p["product-id"]);
        const pricingMap = await getBatchPricing(productIds);

        // Merge products with their pricing
        return products.map((product) => {
            const pricing = pricingMap[product["product-id"]];
            return {
                ...product,
                pricing,
                bestPrice: pricing ? getBestPrice(pricing, options) : undefined,
            };
        });
    } catch (error) {
        console.error('Failed to fetch recommended products:', error);
        throw error;
    }
};

