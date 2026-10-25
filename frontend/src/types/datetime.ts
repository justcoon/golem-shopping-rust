/**
 * Represents a point in time, formatted as an ISO 8601 string or an object with a timestamp property.
 */
export type DateTime = string | { timestamp: string };

/**
 * Converts a JavaScript Date object to a DateTime ISO string
 */
export function dateToDateTime(date: Date): DateTime {
  return date.toISOString();
}

/**
 * Converts a DateTime (ISO string or { timestamp: string }) to a JavaScript Date object
 */
export function dateTimeToDate(dateTime: DateTime | null | undefined): Date {
  if (!dateTime) {
    return new Date();
  }
  if (typeof dateTime === "string") {
    return new Date(dateTime);
  }
  if (typeof dateTime === "object" && "timestamp" in dateTime) {
    return new Date(dateTime.timestamp);
  }
  return new Date(dateTime as any);
}
