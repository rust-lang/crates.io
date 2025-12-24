import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

const schema = v.pipe(
  v.object({
    // `v.string()` is used to support some of our old fixtures that use strings here for some reason
    id: v.optional(v.union([v.number(), v.string()])),

    name: v.optional(v.string()),
    description: v.optional(v.string()),
    downloads: v.optional(v.number()),
    recent_downloads: v.optional(v.number()),
    documentation: v.optional(v.nullable(v.string()), null),
    homepage: v.optional(v.nullable(v.string()), null),
    repository: v.optional(v.nullable(v.string()), null),
    created_at: v.optional(v.string(), '2010-06-16T21:30:45Z'),
    updated_at: v.optional(v.string(), '2017-02-24T12:34:56Z'),
    badges: v.optional(v.array(v.any()), []),
    _extra_downloads: v.optional(v.array(v.any()), []),
    trustpubOnly: v.optional(v.boolean(), false),

    categories: v.optional(v.array(v.any()), []),
    keywords: v.optional(v.array(v.any()), []),
  }),
  v.transform(function (input) {
    let counter = counters.increment('crate');
    let id = input.id ?? counter;
    let name = input.name ?? `crate-${id}`;
    let description = input.description ?? `This is the description for the crate called "${name}"`;
    let downloads = input.downloads ?? (((id + 13) * 42) % 13) * 12_345;
    let recent_downloads = input.recent_downloads ?? (((id + 7) * 31) % 13) * 321;
    return { ...input, id, name, description, downloads, recent_downloads };
  }),
);

const collection = new Collection({ schema });

export default collection;
