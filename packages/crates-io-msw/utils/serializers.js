import { underscore } from './strings.js';

/**
 * @param {{ [s: string]: any; }} model
 */
export function serializeModel(model) {
  /** @type {{ [s: string]: any; }} */
  let json = {};
  for (let [key, value] of Object.entries(model)) {
    json[underscore(key)] = value;
  }
  return json;
}
