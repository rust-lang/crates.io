import { underscore } from './strings.js';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function serializeModel(model: Record<string, unknown>): Record<string, any> {
  let json: Record<string, unknown> = {};
  for (let [key, value] of Object.entries(model)) {
    json[underscore(key)] = value;
  }
  return json;
}
