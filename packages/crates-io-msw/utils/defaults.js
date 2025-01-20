export function applyDefault(attrs, key, fn) {
  if (!(key in attrs)) {
    attrs[key] = fn();
  }
}
