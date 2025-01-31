import { primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

const ORGS = ['rust-lang', 'emberjs', 'rust-random', 'georust', 'actix'];

export default {
  id: primaryKey(Number),

  name: String,
  org: String,
  login: String,
  url: String,
  avatar: String,

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'name', () => `team-${attrs.id}`);
    applyDefault(attrs, 'org', () => ORGS[(attrs.id - 1) % ORGS.length]);
    applyDefault(attrs, 'login', () => `github:${attrs.org}:${attrs.name}`);
    applyDefault(attrs, 'url', () => `https://github.com/${attrs.org}`);
    applyDefault(attrs, 'avatar', () => 'https://avatars1.githubusercontent.com/u/14631425?v=4');
  },
};
