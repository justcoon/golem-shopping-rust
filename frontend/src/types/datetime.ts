/**
 * Represents a point in time as an ISO-8601 UTC timestamp string.
 */
export type DateTime = string;

/**
 * Legacy representation with seconds and nanoseconds precision for backwards compatibility.
 */
export interface LegacyDateTime {
  seconds: number;
  nanoseconds: number;
}

/**
 * Converts a JavaScript Date object to an ISO-8601 DateTime string.
 */
export function dateToDateTime(date: Date): DateTime {
  return date.toISOString();
}

/**
 * Converts a DateTime string (or legacy DateTime object / Date) to a JavaScript Date object.
 */
export function dateTimeToDate(
  dateTime: DateTime | LegacyDateTime | Date | null | undefined,
): Date {
  if (!dateTime) return new Date(NaN);
  if (dateTime instanceof Date) return dateTime;
  if (typeof dateTime === "string") return new Date(dateTime);
  return new Date(dateTime.seconds * 1000 + Math.floor(dateTime.nanoseconds / 1_000_000));
}

/**
 * Formats a DateTime, Date, or ISO string for user-facing display.
 */
export function formatDate(
  d: DateTime | LegacyDateTime | Date | null | undefined,
  options?: Intl.DateTimeFormatOptions,
): string {
  const date = dateTimeToDate(d);
  if (isNaN(date.getTime())) return "—";
  return date.toLocaleDateString(
    "en-US",
    options ?? {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    },
  );
}
