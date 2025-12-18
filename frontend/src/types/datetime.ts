/**
 * Represents a point in time with seconds and nanoseconds precision
 */
export interface DateTime {
  timestamp: string;
}

/**
 * Converts a JavaScript Date object to a DateTime
 */
export function dateToDateTime(date: Date): DateTime {
   return {
     timestamp: date.toISOString(),
   };
}

/**
 * Converts a DateTime to a JavaScript Date object
 */
export function dateTimeToDate(dateTime: DateTime): Date {
  return new Date(dateTime.timestamp);
}
