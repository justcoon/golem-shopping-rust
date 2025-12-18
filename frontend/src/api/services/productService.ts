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
  "product-id": string;
  name: string;
  brand: string;
  description: string;
  tags: string[];
  pricing?: Pricing;
  bestPrice?: number;
}

const enhanceWithPricing = async (
  product: Product,
  options?: PriceFilterOptions,
): Promise<Product> => {
  try {
    const pricing = await getProductPricing(product["product-id"]);
    return {
      ...product,
      pricing,
      bestPrice: getBestPrice(pricing, options),
    };
  } catch (error) {
    console.error(
      `Error enhancing product ${product["product-id"]} with pricing:`,
      error,
    );
    return product; // Return product without pricing if there's an error
  }
};

export const searchProducts = async (
  query: string,
  options?: PriceFilterOptions,
): Promise<Product[]> => {
  try {
    const response = await apiClient.get(
      `/v1/product/search?query=${encodeURIComponent(query)}`,
    );
    const products: Product[] = response.ok;

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
    const response = await apiClient.get(`/v1/product/${productId}`);
    const product: Product = response.ok;

    if (includePricing) {
      return enhanceWithPricing(product, options);
    }

    return product;
  } catch (error) {
    console.error(`Error fetching product ${productId}:`, error);
    throw error;
  }
};


// Helper to get a product with pricing (alias for backward compatibility)
// Get product with pricing (with optional currency and zone filters)
export const getProductWithPricing = (
  productId: string,
  options?: PriceFilterOptions,
): Promise<Product> => getProductById(productId, true, options);

// Get multiple products by IDs with their pricing
export const getProductsByIds = async (
    productIds: string[],
    options?: PriceFilterOptions,
): Promise<Product[]> => {
  try {
    // Get all products, handling individual failures
    const productPromises = productIds.map(id =>
        getProductById(id, false)
            .then(product => ({ success: true, product }))
            .catch(error => {
              console.warn(`Failed to fetch product ${id}:`, error.message);
              return { success: false, id, error };
            })
    );

    const results = await Promise.all(productPromises);

    // Filter out failed fetches and extract successful products
    const validProducts = results
        .filter((result): result is { success: true; product: Product } => result.success)
        .map(result => result.product);

    if (validProducts.length === 0) {
      console.warn('No products were successfully fetched');
      return [];
    }

    // Get pricing only for successfully fetched products
    const validProductIds = validProducts.map(p => p["product-id"]);
    const pricingMap = await getBatchPricing(validProductIds).catch(error => {
      console.warn('Failed to fetch batch pricing, continuing without pricing', error);
      return {}; // Return empty pricing map if batch pricing fails
    });

    // Merge products with their pricing
    const result: Record<string, Product> = {};
    validProducts.forEach((product) => {
      const pricing = pricingMap[product["product-id"]];
      result[product["product-id"]] = {
        ...product,
        pricing,
        bestPrice: pricing ? getBestPrice(pricing, options) : undefined,
      };
    });

    return Object.values(result);
  } catch (error) {
    console.error('Unexpected error in getProductsByIds:', error);
    throw error; // Re-throw for the caller to handle
  }
};

export const getProductBestPrice = (
  product: Product,
  options?: PriceFilterOptions,
): string => {
  if (!product.pricing) return "0.00";
  const price = getBestPrice(product.pricing, options);
  return price?.toFixed(2) || "0.00";
};

export const getProductOriginalPrice = (
  product: Product,
  options?: PriceFilterOptions,
): string => {
  if (!product.pricing) return getProductBestPrice(product, options);

  // Filter list prices by currency and zone if provided
  let listPrices = [...product.pricing["list-prices"]];
  if (options?.currency) {
    listPrices = listPrices.filter((p) => p.currency === options.currency);
  }
  if (options?.zone) {
    listPrices = listPrices.filter((p) => p.zone === options.zone);
  }

  if (listPrices.length > 0) {
    const minListPrice = Math.min(...listPrices.map((p) => p.price));
    return minListPrice.toFixed(2);
  }

  return getProductBestPrice(product, options);
};

export const isProductOnSale = (
  product: Product,
  options?: PriceFilterOptions,
): boolean => {
  if (!product.pricing?.["sale-prices"]?.length) return false;

  // Filter sale prices by date, currency and zone
  const now = new Date();
  const salePrices = product.pricing["sale-prices"].filter(
    (sale: SalePricingItem) => {
      const start = sale.start ? dateTimeToDate(sale.start) : null;
      const end = sale.end ? dateTimeToDate(sale.end) : null;
      const matchesCurrency = options?.currency
        ? sale.currency === options.currency
        : true;
      const matchesZone = options?.zone ? sale.zone === options.zone : true;

      return (
        matchesCurrency &&
        matchesZone &&
        (!start || now >= start) &&
        (!end || now <= end)
      );
    },
  );

  if (salePrices.length === 0) return false;

  const bestSalePrice = Math.min(...salePrices.map((s) => s.price));

  // Get filtered list prices for comparison
  let listPrices = [...product.pricing["list-prices"]];
  if (options?.currency) {
    listPrices = listPrices.filter((p) => p.currency === options.currency);
  }
  if (options?.zone) {
    listPrices = listPrices.filter((p) => p.zone === options.zone);
  }

  const minListPrice =
    listPrices.length > 0
      ? Math.min(...listPrices.map((p) => p.price))
      : Infinity;

  return bestSalePrice < minListPrice;
};

export const getProductImage = (product: { name: string }): string => {
  // Generate a consistent hash from the product name for deterministic image selection
  const nameHash = product.name.split("").reduce((acc, char) => {
    return char.charCodeAt(0) + ((acc << 5) - acc);
  }, 0);

  // Use picsum.photos with the hash to get a consistent but varied image per product
  const imageId = Math.abs(nameHash) % 1000;
  return `https://picsum.photos/seed/${imageId}/300/200`;
};
