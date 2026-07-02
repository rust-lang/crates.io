import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../../utils/counters.js';

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    crate: v.optional(v.any(), null),
    namespace: v.optional(v.string(), 'rust-lang'),
    namespace_id: v.optional(v.nullable(v.string()), null),
    project: v.optional(v.string()),
    workflow_filepath: v.optional(v.string(), '.gitlab-ci.yml'),
    environment: v.optional(v.nullable(v.string()), null),
    created_at: v.optional(v.string(), '2023-01-01T00:00:00Z'),
  }),
  v.transform(function (input) {
    let counter = counters.increment('trustpubGitlabConfig');
    let id = input.id ?? counter;
    let project = input.project ?? `repo-${id}`;
    return { ...input, id, project };
  }),
);

const collection = new Collection({ schema });

export default collection;
