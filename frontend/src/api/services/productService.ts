import apiClient from "../config";
import {
  getProductPricing,
  getBatchPricing,
  getBestPrice,
  type Pricing,
  type SalePricingItem,
  type PriceFilterOptions,
} from "./pricingService";
import { dateTimeToDate } from "@/types/datetime.ts";

export interface Product {
  product_id: string;
  name: string;
  brand: string;
  description: string;
  tags: string[];
  created_at?: string;
  updated_at?: string;
  pricing?: Pricing;
  bestPrice?: number;
}

const enhanceWithPricing = async (
  product: Product,
  options?: PriceFilterOptions,
): Promise<Product> => {
  try {
    const pricing = await getProductPricing(product.product_id);
    return {
      ...product,
      pricing,
      bestPrice: getBestPrice(pricing, options) ?? undefined,
    };
  } catch (error) {
    console.error(`Error enhancing product ${product.product_id} with pricing:`, error);
    return product;
  }
};

export const searchProducts = async (
  query: string,
  options?: PriceFilterOptions,
): Promise<Product[]> => {
  try {
    const products: Product[] = await apiClient.get(
      `/v1/product/search?query=${encodeURIComponent(query)}`,
    );

    const productIds = products.map((p) => p.product_id);
    const pricingMap = await getBatchPricing(productIds);

    return products.map((product) => {
      const pricing = pricingMap[product.product_id];
      return {
        ...product,
        pricing,
        bestPrice: pricing ? (getBestPrice(pricing, options) ?? undefined) : undefined,
      };
    });
  } catch (error) {
    console.error("Error searching products:", error);
    throw error;
  }
};

export const getProductById = async (
  productId: string,
  includePricing = true,
  options?: PriceFilterOptions,
): Promise<Product> => {
  try {
    const product: Product = await apiClient.get(`/v1/product/${productId}`);

    if (includePricing) {
      return enhanceWithPricing(product, options);
    }

    return product;
  } catch (error) {
    console.error(`Error fetching product ${productId}:`, error);
    throw error;
  }
};

export const getProductWithPricing = (
  productId: string,
  options?: PriceFilterOptions,
): Promise<Product> => getProductById(productId, true, options);

export const getProductsByIds = async (
  productIds: string[],
  options?: PriceFilterOptions,
): Promise<Product[]> => {
  try {
    const productPromises = productIds.map((id) =>
      getProductById(id, false)
        .then((product) => ({ success: true, product }))
        .catch((error) => {
          console.warn(`Failed to fetch product ${id}:`, error.message);
          return { success: false, id, error };
        }),
    );

    const results = await Promise.all(productPromises);
    const validProducts = results
      .filter((result): result is { success: true; product: Product } => result.success)
      .map((result) => result.product);

    if (validProducts.length === 0) {
      console.warn("No products were successfully fetched");
      return [];
    }

    const validProductIds = validProducts.map((p) => p.product_id);
    const pricingMap = await getBatchPricing(validProductIds).catch((error) => {
      console.warn("Failed to fetch batch pricing, continuing without pricing", error);
      return {};
    });

    const result: Record<string, Product> = {};
    validProducts.forEach((product) => {
      const pricing = pricingMap[product.product_id];
      result[product.product_id] = {
        ...product,
        pricing,
        bestPrice: pricing ? (getBestPrice(pricing, options) ?? undefined) : undefined,
      };
    });

    return Object.values(result);
  } catch (error) {
    console.error("Unexpected error in getProductsByIds:", error);
    throw error;
  }
};

export const getProductBestPrice = (product: Product, options?: PriceFilterOptions): string => {
  if (!product.pricing) return "0.00";
  const price = getBestPrice(product.pricing, options);
  return price?.toFixed(2) || "0.00";
};

export const getProductOriginalPrice = (product: Product, options?: PriceFilterOptions): string => {
  if (!product.pricing) return getProductBestPrice(product, options);

  let listPrices = [...product.pricing.list_prices];
  if (options?.currency) {
    listPrices = listPrices.filter((p) => p.currency === options.currency);
  }
  if (options?.region) {
    listPrices = listPrices.filter((p) => p.region === options.region);
  }

  if (listPrices.length > 0) {
    const minListPrice = Math.min(...listPrices.map((p) => p.price));
    return minListPrice.toFixed(2);
  }

  return getProductBestPrice(product, options);
};

export const isProductOnSale = (product: Product, options?: PriceFilterOptions): boolean => {
  if (!product.pricing?.sale_prices?.length) return false;

  const now = new Date();
  const salePrices = product.pricing.sale_prices.filter((sale: SalePricingItem) => {
    const start = sale.start ? dateTimeToDate(sale.start) : null;
    const end = sale.end ? dateTimeToDate(sale.end) : null;
    const matchesCurrency = options?.currency ? sale.currency === options.currency : true;
    const matchesRegion = options?.region ? sale.region === options.region : true;

    return matchesCurrency && matchesRegion && (!start || now >= start) && (!end || now <= end);
  });

  if (salePrices.length === 0) return false;

  const bestSalePrice = Math.min(...salePrices.map((s) => s.price));

  let listPrices = [...product.pricing.list_prices];
  if (options?.currency) {
    listPrices = listPrices.filter((p) => p.currency === options.currency);
  }
  if (options?.region) {
    listPrices = listPrices.filter((p) => p.region === options.region);
  }

  const minListPrice =
    listPrices.length > 0 ? Math.min(...listPrices.map((p) => p.price)) : Infinity;

  return bestSalePrice < minListPrice;
};

export const getProductImage = (product: { name: string }): string => {
  const name = product?.name || "";
  const nameHash = name.split("").reduce((acc, char) => {
    return char.charCodeAt(0) + ((acc << 5) - acc);
  }, 0);

  const imageId = Math.abs(nameHash) % 1000;
  return `https://picsum.photos/seed/${imageId}/300/200`;
};
