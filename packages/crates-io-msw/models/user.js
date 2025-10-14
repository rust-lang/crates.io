import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';
import { dasherize } from '../utils/strings.js';

const schema = z.object({
  id: z.number(),

  name: z.string().nullable(),
  login: z.string(),
  url: z.string(),
  avatar: z.string(),
  email: z.string().nullable(),
  emailVerificationToken: z.string().nullable(),
  emailVerified: z.boolean(),
  isAdmin: z.boolean(),
  publishNotifications: z.boolean(),

  followedCrates: z.array(z.any()).default(() => []),
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
