import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';
import { seededRandom } from '../utils/random.js';

const schema = z.object({
  id: z.number(),

  crateScopes: z.array(z.any()).nullable(),
  createdAt: z.string(),
  endpointScopes: z.array(z.any()).nullable(),
  expiredAt: z.string().nullable(),
  lastUsedAt: z.string().nullable(),
  name: z.string(),
  token: z.string(),
  revoked: z.boolean(),

  user: z.any(),
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
