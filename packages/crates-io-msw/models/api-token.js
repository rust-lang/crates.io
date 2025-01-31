import { nullable, oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';
import { seededRandom } from '../utils/random.js';

export default {
  id: primaryKey(Number),

  crateScopes: nullable(Array),
  createdAt: String,
  endpointScopes: nullable(Array),
  expiredAt: nullable(String),
  lastUsedAt: nullable(String),
  name: String,
  token: String,
  revoked: Boolean,

  user: oneOf('user'),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'crateScopes', () => null);
    applyDefault(attrs, 'createdAt', () => '2017-11-19T17:59:22Z');
    applyDefault(attrs, 'endpointScopes', () => null);
    applyDefault(attrs, 'expiredAt', () => null);
    applyDefault(attrs, 'lastUsedAt', () => null);
    applyDefault(attrs, 'name', () => `API Token ${attrs.id}`);
    applyDefault(attrs, 'token', () => generateToken(counter));
    applyDefault(attrs, 'revoked', () => false);

    if (!attrs.user) {
      throw new Error('Missing `user` relationship on `api-token`');
    }
  },
};

function generateToken(seed) {
  return seededRandom(seed).toString().slice(2);
}
