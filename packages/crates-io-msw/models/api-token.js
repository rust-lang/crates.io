import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';
import { seededRandom } from '../utils/random.js';

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    crateScopes: v.optional(v.nullable(v.array(v.string())), null),
    createdAt: v.optional(v.string(), '2017-11-19T17:59:22Z'),
    endpointScopes: v.optional(v.nullable(v.array(v.string())), null),
    expiredAt: v.optional(v.nullable(v.string()), null),
    lastUsedAt: v.optional(v.nullable(v.string()), null),
    name: v.optional(v.string()),
    token: v.optional(v.string()),
    revoked: v.optional(v.boolean(), false),

    user: v.any(),
  }),
  v.transform(function (input) {
    let counter = counters.increment('apiToken');
    let id = input.id ?? counter;
    let name = input.name ?? `API Token ${id}`;
    let token = input.token ?? generateToken(id);
    return { ...input, id, name, token };
  }),
);

function generateToken(seed) {
  return seededRandom(seed).toString().slice(2);
}

const collection = new Collection({ schema });

export default collection;
