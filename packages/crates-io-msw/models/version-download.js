import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    date: v.optional(v.string(), '2019-05-21'),
    downloads: v.optional(v.number()),

    version: v.any(),
  }),
  v.transform(function (input) {
    let counter = counters.increment('versionDownload');
    let id = input.id ?? counter;
    let downloads = input.downloads ?? (((id + 13) * 42) % 13) * 2345;
    return { ...input, id, downloads };
  }),
);

const collection = new Collection({ schema });

export default collection;
