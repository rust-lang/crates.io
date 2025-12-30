/**
 * Truncates text to a maximum length, appending "..." if truncated.
 */
export function truncateText(value: string, maxLength = 200): string {
  if (value.length > maxLength) {
    return `${value.slice(0, maxLength).trim()} …`;
  }
  return value;
}
