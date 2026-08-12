import { Collection } from '@msw/data';
import * as v from 'valibot';

import * as counters from '../utils/counters.js';
import { dasherize } from '../utils/strings.js';

const githubAccountSchema = v.object({
  accountId: v.string(),
  login: v.string(),
  avatar: v.nullable(v.string()),
});

const schema = v.pipe(
  v.object({
    id: v.optional(v.number()),

    name: v.optional(v.nullable(v.string())),
    login: v.optional(v.string()),
    githubAccounts: v.optional(v.array(githubAccountSchema)),
    url: v.optional(v.string()),
    avatar: v.optional(v.string(), 'https://avatars1.githubusercontent.com/u/14631425?v=4'),
    email: v.optional(v.nullable(v.string())),
    emailVerificationToken: v.optional(v.nullable(v.string()), null),
    emailVerified: v.optional(v.boolean()),
    isAdmin: v.optional(v.boolean(), false),
    publishNotifications: v.optional(v.boolean(), true),

    followedCrates: v.optional(v.array(v.any()), []),
  }),
  v.transform(function (input) {
    let counter = counters.increment('user');
    let id = input.id ?? counter;
    let name = input.name === undefined ? `User ${id}` : input.name;
    let login = input.login ?? (name ? dasherize(name) : `user-${id}`);
    let email = input.email === undefined ? `${login}@crates.io` : input.email;
    let githubAccounts = input.githubAccounts ?? [
      {
        accountId: String(id),
        login,
        avatar: input.avatar ?? null,
      },
    ];
    let githubProfileLogin = githubAccounts[0]?.login ?? login;
    let url = input.url ?? `https://github.com/${githubProfileLogin}`;
    let emailVerified = input.emailVerified ?? Boolean(email && !input.emailVerificationToken);
    return { ...input, id, name, login, githubAccounts, email, url, emailVerified };
  }),
);

const collection = new Collection({ schema });

export default collection;
