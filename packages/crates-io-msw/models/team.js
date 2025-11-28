import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const ORGS = ['rust-lang', 'emberjs', 'rust-random', 'georust', 'actix'];

const schema = z.object({
  id: z.number(),

  name: z.string(),
  org: z.string(),
  login: z.string(),
  url: z.string(),
  avatar: z.string(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);
  applyDefault(attrs, 'name', () => `team-${attrs.id}`);
  applyDefault(attrs, 'org', () => ORGS[(attrs.id - 1) % ORGS.length]);
  applyDefault(attrs, 'login', () => `github:${attrs.org}:${attrs.name}`);
  applyDefault(attrs, 'url', () => `https://github.com/${attrs.org}`);
  applyDefault(attrs, 'avatar', () => 'https://avatars1.githubusercontent.com/u/14631425?v=4');
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
