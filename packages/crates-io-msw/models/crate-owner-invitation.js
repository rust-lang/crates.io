import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    createdAt: v.optional(v.string(), '2016-12-24T12:34:56Z'),
    expiresAt: v.optional(v.string(), '2017-01-24T12:34:56Z'),
    token: v.optional(v.string()),

    crate: v.any(),
    invitee: v.any(),
    inviter: v.any(),
  }),
  v.transform(function (input) {
    let counter = counters.increment('crateOwnerInvitation');
    let id = input.id ?? counter;
    let token = input.token ?? `secret-token-${id}`;
    return { ...input, id, token };
  }),
);

const collection = new Collection({ schema });

export default collection;
