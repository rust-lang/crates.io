import { underscore } from './strings.js';

export function serializeModel(model) {
  let json = {};
  for (let [key, value] of Object.entries(model)) {
    json[underscore(key)] = value;
  }
  return json;
}
