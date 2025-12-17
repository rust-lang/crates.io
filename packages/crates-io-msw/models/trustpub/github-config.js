import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../../utils/counters.js';

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    crate: v.optional(v.any(), null),
    repository_owner: v.optional(v.string(), 'rust-lang'),
    repository_owner_id: v.optional(v.number(), 5_430_905),
    repository_name: v.optional(v.string()),
    workflow_filename: v.optional(v.string(), 'ci.yml'),
    environment: v.optional(v.nullable(v.string()), null),
    created_at: v.optional(v.string(), '2023-01-01T00:00:00Z'),
  }),
  v.transform(function (input) {
    let counter = counters.increment('trustpubGithubConfig');
    let id = input.id ?? counter;
    let repository_name = input.repository_name ?? `repo-${id}`;
    return { ...input, id, repository_name };
  }),
);

const collection = new Collection({ schema });

export default collection;
