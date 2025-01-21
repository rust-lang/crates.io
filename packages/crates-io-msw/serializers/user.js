import { serializeModel } from '../utils/serializers.js';

export function serializeUser(user, { removePrivateData = true } = {}) {
  let serialized = serializeModel(user);

  if (removePrivateData) {
    delete serialized.email;
    delete serialized.email_verified;
    delete serialized.is_admin;
    delete serialized.publish_notifications;
  } else {
    serialized.email_verification_sent = serialized.email_verified || Boolean(serialized.email_verification_token);
  }

  delete serialized.email_verification_token;
  delete serialized.followed_crates;

  return serialized;
}
