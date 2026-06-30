/**
 * @param {string} str
 */
export function dasherize(str) {
  return str
    .replaceAll(/([a-z\d])([A-Z])/g, '$1_$2')
    .toLowerCase()
    .replaceAll(/[ _]/g, '-');
}

/**
 * @param {string} str
 */
export function underscore(str) {
  return str
    .replaceAll(/([a-z\d])([A-Z]+)/g, '$1_$2')
    .replaceAll(/-|\s+/g, '_')
    .toLowerCase();
}
