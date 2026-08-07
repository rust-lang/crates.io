import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

/**
 * MSW-only pending signup state used because request handlers cannot access
 * the signed session cookie used by the real API.
 */
const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    login: v.string(),
    email: v.optional(v.nullable(v.string()), null),
  }),
  v.transform(function (input) {
    let counter = counters.increment('pendingSignup');
    let id = input.id ?? counter;
    return { ...input, id };
  }),
);

const collection = new Collection({ schema });

export default collection;
