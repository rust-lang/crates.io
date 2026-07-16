import { underscore } from './strings.js';

export function serializeModel(model: Record<string, unknown>) {
  let json: Record<string, unknown> = {};
  for (let [key, value] of Object.entries(model)) {
    json[underscore(key)] = value;
  }
  return json;
}
