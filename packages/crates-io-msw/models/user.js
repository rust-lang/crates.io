import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';
import { dasherize } from '../utils/strings.js';

const schema = v.object({
  id: v.number(),

  name: v.nullable(v.string()),
  login: v.string(),
  url: v.string(),
  avatar: v.string(),
  email: v.nullable(v.string()),
  emailVerificationToken: v.nullable(v.string()),
  emailVerified: v.boolean(),
  isAdmin: v.boolean(),
  publishNotifications: v.boolean(),

  followedCrates: v.optional(v.array(v.any()), () => []),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);
  applyDefault(attrs, 'name', () => `User ${attrs.id}`);
  applyDefault(attrs, 'login', () => (attrs.name ? dasherize(attrs.name) : `user-${attrs.id}`));
  applyDefault(attrs, 'email', () => `${attrs.login}@crates.io`);
  applyDefault(attrs, 'url', () => `https://github.com/${attrs.login}`);
  applyDefault(attrs, 'avatar', () => 'https://avatars1.githubusercontent.com/u/14631425?v=4');
  applyDefault(attrs, 'emailVerificationToken', () => null);
  applyDefault(attrs, 'emailVerified', () => Boolean(attrs.email && !attrs.emailVerificationToken));
  applyDefault(attrs, 'isAdmin', () => false);
  applyDefault(attrs, 'publishNotifications', () => true);
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
