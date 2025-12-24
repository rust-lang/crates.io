import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

/**
 * This is a MSW-only model, that is used to keep track of the current
 * session and the associated `user` model, because in route handlers we don't
 * have access to the cookie data that the actual API is using for these things.
 *
 * This mock implementation means that there can only ever exist one
 * session at a time.
 */
const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    user: v.any(),
  }),
  v.transform(function (input) {
    let counter = counters.increment('mswSession');
    let id = input.id ?? counter;
    return { ...input, id };
  }),
);

const collection = new Collection({ schema });

export default collection;
