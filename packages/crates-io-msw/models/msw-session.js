import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

/**
 * This is a MSW-only model, that is used to keep track of the current
 * session and the associated `user` model, because in route handlers we don't
 * have access to the cookie data that the actual API is using for these things.
 *
 * This mock implementation means that there can only ever exist one
 * session at a time.
 */
const schema = z.object({
  id: z.number(),

  user: z.any(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);

  if (!attrs.user) {
    throw new Error('Missing `user` relationship');
  }
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
