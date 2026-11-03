/**
 * Format a timestamp to a locale-adapted string in the local time zone, such as
 * "October 16, 2026 at 3:30 PM PDT" or
 * "17. Oktober 2026 um 00:30 MESZ".
 */
export function dateToLocaleString(date: string | Date): string {
  let formatter = new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    day: 'numeric',
    month: 'long',
    hour: 'numeric',
    minute: 'numeric',
    timeZoneName: 'short',
  });
  return formatter.format(new Date(date));
}
