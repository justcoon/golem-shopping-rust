import apiClient from "../config";
import {getProductsByIds, type Product} from "./productService.ts";
import {PriceFilterOptions} from "@/api/services/pricingService.ts";

export interface RecommendedItems {
    "product_ids": string[];
    "product_brands": string[];
}

export const getRecommendedProducts = async (userId: string, options?: PriceFilterOptions): Promise<Product[]> => {
    try {
        const recommendedItems: RecommendedItems = await apiClient.get(`/v1/assistant/${userId}/recommended_items`);
        return await getProductsByIds(recommendedItems["product_ids"], options);
    } catch (error) {
        console.error('Failed to fetch recommended products:', error);
        throw error;
    }
};

