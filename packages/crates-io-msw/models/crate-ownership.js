import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    emailNotifications: v.optional(v.boolean(), true),

    crate: v.any(),
    team: v.optional(v.nullable(v.any()), null),
    user: v.optional(v.nullable(v.any()), null),
  }),
  v.transform(function (input) {
    let counter = counters.increment('crateOwnership');
    let id = input.id ?? counter;
    return { ...input, id };
  }),
  v.check(input => input.crate != null, 'Missing `crate` relationship on `crate-ownership`'),
  v.check(input => input.team != null || input.user != null, 'Missing `team` or `user` relationship on `crate-ownership`'),
  v.check(input => !(input.team != null && input.user != null), '`team` and `user` on a `crate-ownership` are mutually exclusive'),
);

const collection = new Collection({ schema });

export default collection;
