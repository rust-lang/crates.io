/**
 * Format a timestamp to a locale-adapted string with the UTC time zone, such as
 * "October 16, 2026 at 10:00 PM UTC" or
 * "16. Oktober 2026 um 22:00 UTC".
 */
export function dateToLocaleStringUTC(date: string | Date): string {
  if (!(date instanceof Date)) {
    // Accept dates or strings
    date = new Date(date);
  }
  let formatter = new Intl.DateTimeFormat(undefined, {
    timeZone: 'UTC',
    year: 'numeric',
    day: 'numeric',
    month: 'long',
    hour: 'numeric',
    minute: 'numeric',
    timeZoneName: 'short',
  });
  return formatter.format(date);
}
