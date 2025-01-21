export function dasherize(str) {
  return str
    .replace(/([a-z\d])([A-Z])/g, '$1_$2')
    .toLowerCase()
    .replace(/[ _]/g, '-');
}

export function underscore(str) {
  return str
    .replace(/([a-z\d])([A-Z]+)/g, '$1_$2')
    .replace(/-|\s+/g, '_')
    .toLowerCase();
}
