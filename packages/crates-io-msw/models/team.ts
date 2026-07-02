import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';

const ORGS = ['rust-lang', 'emberjs', 'rust-random', 'georust', 'actix'];

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    name: v.optional(v.string()),
    org: v.optional(v.string()),
    login: v.optional(v.string()),
    url: v.optional(v.string()),
    avatar: v.optional(v.string(), 'https://avatars1.githubusercontent.com/u/14631425?v=4'),
  }),
  v.transform(function (input) {
    let counter = counters.increment('team');
    let id = input.id ?? counter;
    let name = input.name ?? `team-${id}`;
    let org = input.org ?? ORGS[(id - 1) % ORGS.length];
    let login = input.login ?? `github:${org}:${name}`;
    let url = input.url ?? `https://github.com/${org}`;
    return { ...input, id, name, org, login, url };
  }),
);

const collection = new Collection({ schema });

export default collection;
