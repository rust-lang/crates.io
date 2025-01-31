import { oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

export default {
  id: primaryKey(Number),

  createdAt: String,
  expiresAt: String,
  token: String,

  crate: oneOf('crate'),
  invitee: oneOf('user'),
  inviter: oneOf('user'),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'createdAt', () => '2016-12-24T12:34:56Z');
    applyDefault(attrs, 'expiresAt', () => '2017-01-24T12:34:56Z');
    applyDefault(attrs, 'token', () => `secret-token-${attrs.id}`);

    if (!attrs.crate) {
      throw new Error(`Missing \`crate\` relationship on \`crate-owner-invitation\``);
    }
    if (!attrs.invitee) {
      throw new Error(`Missing \`invitee\` relationship on \`crate-owner-invitation\``);
    }
    if (!attrs.inviter) {
      throw new Error(`Missing \`inviter\` relationship on \`crate-owner-invitation\``);
    }
  },
};
