import { serializeModel } from '../utils/serializers.js';
import { serializeEmail } from './email.js';

export function serializeUser(user, { removePrivateData = true } = {}) {
  let serialized = serializeModel(user);
  serialized.emails = user.emails.map(email => serializeEmail(email));

  if (removePrivateData) {
    delete serialized.emails;
    delete serialized.is_admin;
    delete serialized.publish_notifications;
  }

  delete serialized.followed_crates;

  return serialized;
}
