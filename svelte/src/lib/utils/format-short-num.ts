const DEFAULT_THRESHOLD = 1500;
const UNITS = ['', 'K', 'M', 'B'];

interface FormatShortNumOptions {
  /**
   * Value at or above which the number rolls over to the next unit. Defaults
   * to {@link DEFAULT_THRESHOLD}.
   */
  threshold?: number;
}

/**
 * Formats a number in a compact form with K/M/B suffix.
 *
 * With the default threshold this matches the implementation in https://github.com/rust-lang/crates_io_og_image/blob/v0.2.1/src/formatting.rs
 * to ensure that we render roughly the same values in our user interface and the generated OpenGraph images.
 */
export function formatShortNum(value: number, { threshold = DEFAULT_THRESHOLD }: FormatShortNumOptions = {}): string {
  let unitIndex = 0;

  // Keep dividing by 1000 until value is below threshold or we've reached the last unit
  while (value >= threshold && unitIndex < UNITS.length - 1) {
    value /= 1000;
    unitIndex += 1;
  }

  let unit = UNITS[unitIndex];

  // Special case for numbers without suffix - no decimal places
  if (unitIndex === 0) {
    return value.toLocaleString();
  }

  // For K and M, format with appropriate decimal places
  // Determine number of decimal places to keep number under 4 chars
  let fractionDigits = value < 10 ? 1 : 0;
  let number = value.toLocaleString(undefined, {
    minimumFractionDigits: fractionDigits,
    maximumFractionDigits: fractionDigits,
  });

  return number + unit;
}
