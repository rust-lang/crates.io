import { serializeModel } from '../utils/serializers.js';

export function serializeApiToken(token, { forCreate = false } = {}) {
  let serialized = serializeModel(token);

  if (serialized.created_at) {
    serialized.created_at = new Date(serialized.created_at).toISOString();
  }
  if (serialized.expired_at) {
    serialized.expired_at = new Date(serialized.expired_at).toISOString();
  }
  if (serialized.last_used_at) {
    serialized.last_used_at = new Date(serialized.last_used_at).toISOString();
  }

  delete serialized.user;

  if (!forCreate) {
    delete serialized.revoked;
    delete serialized.token;
  }

  return serialized;
}
