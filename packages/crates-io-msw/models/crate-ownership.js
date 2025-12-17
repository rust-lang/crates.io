import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const schema = v.object({
  id: v.number(),

  emailNotifications: v.boolean(),

  crate: v.any(),
  team: v.any(),
  user: v.any(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);
  applyDefault(attrs, 'emailNotifications', () => true);
  applyDefault(attrs, 'team', () => null);
  applyDefault(attrs, 'user', () => null);

  if (!attrs.crate) {
    throw new Error('Missing `crate` relationship on `crate-ownership`');
  }
  if (!attrs.team && !attrs.user) {
    throw new Error('Missing `team` or `user` relationship on `crate-ownership`');
  }
  if (attrs.team && attrs.user) {
    throw new Error('`team` and `user` on a `crate-ownership` are mutually exclusive');
  }
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
