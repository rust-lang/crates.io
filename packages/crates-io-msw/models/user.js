import { manyOf, nullable, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';
import { dasherize } from '../utils/strings.js';

export default {
  id: primaryKey(Number),

  name: nullable(String),
  login: String,
  url: String,
  avatar: String,
  emails: manyOf('email'),
  isAdmin: Boolean,
  publishNotifications: Boolean,

  followedCrates: manyOf('crate'),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'name', () => `User ${attrs.id}`);
    applyDefault(attrs, 'login', () => (attrs.name ? dasherize(attrs.name) : `user-${attrs.id}`));
    applyDefault(attrs, 'url', () => `https://github.com/${attrs.login}`);
    applyDefault(attrs, 'avatar', () => 'https://avatars1.githubusercontent.com/u/14631425?v=4');
    applyDefault(attrs, 'isAdmin', () => false);
    applyDefault(attrs, 'publishNotifications', () => true);
  },
};
