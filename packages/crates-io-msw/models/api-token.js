import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';
import { seededRandom } from '../utils/random.js';

const schema = v.object({
  id: v.number(),

  crateScopes: v.nullable(v.array(v.any())),
  createdAt: v.string(),
  endpointScopes: v.nullable(v.array(v.any())),
  expiredAt: v.nullable(v.string()),
  lastUsedAt: v.nullable(v.string()),
  name: v.string(),
  token: v.string(),
  revoked: v.boolean(),

  user: v.any(),
});

function preCreate(attrs, counter) {
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
}

function generateToken(seed) {
  return seededRandom(seed).toString().slice(2);
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
