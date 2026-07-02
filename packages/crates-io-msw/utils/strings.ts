export function dasherize(str: string) {
  return str
    .replaceAll(/([a-z\d])([A-Z])/g, '$1_$2')
    .toLowerCase()
    .replaceAll(/[ _]/g, '-');
}

export function underscore(str: string) {
  return str
    .replaceAll(/([a-z\d])([A-Z]+)/g, '$1_$2')
    .replaceAll(/-|\s+/g, '_')
    .toLowerCase();
}
