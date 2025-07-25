import { serializeModel } from '../utils/serializers.js';

export function serializeEmail(email) {
  let serialized = serializeModel(email);

  delete serialized.token;

  return serialized;
}
