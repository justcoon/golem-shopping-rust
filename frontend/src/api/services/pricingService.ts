import apiClient from "../config";
import { DateTime, dateTimeToDate } from "@/types/datetime.ts";

export interface PricingItem {
  price: number;
  currency: string;
  region: string;
}

export interface SalePricingItem extends PricingItem {
  start?: DateTime;
  end?: DateTime;
}

export interface Pricing {
  product_id: string;
  msrp_prices: PricingItem[];
  list_prices: PricingItem[];
  sale_prices: SalePricingItem[];
  created_at: DateTime;
  updated_at: DateTime;
}

export const getProductPricing = async (productId: string): Promise<Pricing> => {
  try {
    return await apiClient.get(`/v1/pricing/${productId}`);
  } catch (error) {
    console.error(`Error fetching pricing for product ${productId}:`, error);
    throw error;
  }
};

export const getBatchPricing = async (productIds: string[]): Promise<Record<string, Pricing>> => {
  try {
    const pricingPromises = productIds.map(async (id) => {
      try {
        const pricing = await getProductPricing(id);
        return { id, pricing };
      } catch (error) {
        console.error(`Error fetching pricing for product ${id}:`, error);
        return { id, error };
      }
    });

    const results = await Promise.all(pricingPromises);

    return results.reduce<Record<string, Pricing>>((acc, { id, pricing }) => {
      if (pricing) {
        acc[id] = pricing;
      }
      return acc;
    }, {});
  } catch (error) {
    console.error("Error in batch pricing operation:", error);
    throw error;
  }
};

export interface PriceFilterOptions {
  currency?: string;
  region?: string;
}

export const getCurrentSalePrices = (
  pricing: Pricing,
  options?: PriceFilterOptions | string,
): SalePricingItem[] => {
  const now = new Date();
  const filterOptions =
    typeof options === "string"
      ? { currency: options }
      : options || {};

  return pricing.sale_prices.filter((sale) => {
    const start = sale.start ? dateTimeToDate(sale.start) : null;
    const end = sale.end ? dateTimeToDate(sale.end) : null;
    const matchesCurrency = filterOptions.currency
      ? sale.currency === filterOptions.currency
      : true;
    const matchesRegion = filterOptions.region ? sale.region === filterOptions.region : true;

    return matchesCurrency && matchesRegion && (!start || now >= start) && (!end || now <= end);
  });
};

export const getBestPrice = (
  pricing: Pricing,
  options?: PriceFilterOptions | string,
): number | null => {
  const filterOptions =
    typeof options === "string"
      ? { currency: options }
      : options || {};

  const salePrices = getCurrentSalePrices(pricing, filterOptions);
  const listPrices = pricing.list_prices.filter((p) => {
    const matchesCurrency = filterOptions.currency ? p.currency === filterOptions.currency : true;
    const matchesRegion = filterOptions.region ? p.region === filterOptions.region : true;
    return matchesCurrency && matchesRegion;
  });

  if (salePrices.length > 0) {
    return Math.min(...salePrices.map((p) => p.price));
  } else if (listPrices.length > 0) {
    return Math.min(...listPrices.map((p) => p.price));
  }

  return null;
};
