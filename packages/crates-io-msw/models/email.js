import { nullable, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

export default {
  id: primaryKey(Number),

  email: String,
  verified: Boolean,
  verification_email_sent: Boolean,
  primary: Boolean,
  token: nullable(String),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'email', () => `foo@crates.io`);
    applyDefault(attrs, 'verified', () => false);
    applyDefault(attrs, 'verification_email_sent', () => false);
    applyDefault(attrs, 'primary', () => false);
    applyDefault(attrs, 'token', () => null);
  },
};
