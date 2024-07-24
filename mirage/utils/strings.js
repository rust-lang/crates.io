// Imported from the `@ember/string` package to avoid build issues...

export function decamelize(str) {
  return str.replace(/([\da-z])([A-Z])/g, '$1_$2').toLowerCase();
}

export function dasherize(str) {
  return decamelize(str).replace(/[ _]/g, '-');
}
