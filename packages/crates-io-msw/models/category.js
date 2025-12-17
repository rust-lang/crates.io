import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';
import { dasherize } from '../utils/strings.js';

const schema = v.pipe(
  v.object({
    id: v.optional(v.string()),

    category: v.optional(v.string()),
    slug: v.optional(v.string()),
    description: v.optional(v.string()),
    created_at: v.optional(v.string(), '2010-06-16T21:30:45Z'),
    crates_cnt: v.optional(v.nullable(v.number()), null),
  }),
  v.transform(function (input) {
    let counter = counters.increment('category');
    let category = input.category ?? `Category ${counter}`;
    let slug = input.slug ?? dasherize(category);
    let id = input.id ?? slug;
    let description = input.description ?? `This is the description for the category called "${category}"`;
    return { ...input, id, category, slug, description };
  }),
);

const collection = new Collection({ schema });

export default collection;
