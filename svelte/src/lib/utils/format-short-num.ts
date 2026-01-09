const THRESHOLD = 1500;
const UNITS = ['', 'K', 'M'];

/**
 * Formats a number in a compact form with K/M suffix.
 *
 * This matches the implementation in https://github.com/rust-lang/crates_io_og_image/blob/v0.2.1/src/formatting.rs
 * to ensure that we render roughly the same values in our user interface and the generated OpenGraph images.
 */
export function formatShortNum(value: number): string {
  let unitIndex = 0;

  // Keep dividing by 1000 until value is below threshold or we've reached the last unit
  while (value >= THRESHOLD && unitIndex < UNITS.length - 1) {
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
