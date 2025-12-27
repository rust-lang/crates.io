import Helper from '@ember/component/helper';
import { service } from '@ember/service';

const THRESHOLD = 1500;
const UNITS = ['', 'K', 'M'];

/**
 * This matches the implementation in https://github.com/rust-lang/crates_io_og_image/blob/v0.2.1/src/formatting.rs
 * to ensure that we render roughly the same values in our user interface and the generated OpenGraph images.
 */
export default class FormatShortNumHelper extends Helper {
  @service intl;

  compute([value]) {
    let numValue = Number(value);
    let unitIndex = 0;

    // Keep dividing by 1000 until value is below threshold or we've reached the last unit
    while (numValue >= THRESHOLD && unitIndex < UNITS.length - 1) {
      numValue /= 1000;
      unitIndex += 1;
    }

    let unit = UNITS[unitIndex];

    // Special case for numbers without suffix - no decimal places
    if (unitIndex === 0) {
      return this.intl.formatNumber(value);
    }

    // For K and M, format with appropriate decimal places
    // Determine number of decimal places to keep number under 4 chars
    let fractionDigits = numValue < 10 ? 1 : 0;
    let number = this.intl.formatNumber(numValue, {
      minimumFractionDigits: fractionDigits,
      maximumFractionDigits: fractionDigits,
    });

    return number + unit;
  }
}
