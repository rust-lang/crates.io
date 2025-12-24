import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

const REQS = ['^0.1.0', '^2.1.3', '0.3.7', '~5.2.12'];

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    default_features: v.optional(v.boolean()),
    features: v.optional(v.array(v.any()), []),
    kind: v.optional(v.string()),
    optional: v.optional(v.boolean()),
    req: v.optional(v.string()),
    target: v.optional(v.nullable(v.string()), null),

    crate: v.any(),
    version: v.any(),
  }),
  v.transform(function (input) {
    let counter = counters.increment('dependency');
    let id = input.id ?? counter;
    let default_features = input.default_features ?? counter % 4 === 3;
    let kind = input.kind ?? (counter % 3 === 0 ? 'dev' : 'normal');
    let optional = input.optional ?? counter % 4 !== 3;
    let req = input.req ?? REQS[counter % REQS.length];
    return { ...input, id, default_features, kind, optional, req };
  }),
);

const collection = new Collection({ schema });

export default collection;
