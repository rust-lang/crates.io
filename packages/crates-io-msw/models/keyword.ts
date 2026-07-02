import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

const schema = v.pipe(
  v.object({
    id: v.optional(v.string()),
    keyword: v.optional(v.string()),
  }),
  v.transform(function (input) {
    let counter = counters.increment('keyword');
    let keyword = input.keyword ?? `keyword-${counter}`;
    let id = input.id ?? keyword;
    return { ...input, id, keyword };
  }),
);

const collection = new Collection({ schema });

export default collection;
