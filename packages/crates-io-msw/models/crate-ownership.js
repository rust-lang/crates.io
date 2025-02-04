import { nullable, oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

export default {
  id: primaryKey(Number),

  emailNotifications: Boolean,

  crate: oneOf('crate'),
  team: nullable(oneOf('team')),
  user: nullable(oneOf('user')),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'emailNotifications', () => true);

    if (!attrs.crate) {
      throw new Error('Missing `crate` relationship on `crate-ownership`');
    }
    if (!attrs.team && !attrs.user) {
      throw new Error('Missing `team` or `user` relationship on `crate-ownership`');
    }
    if (attrs.team && attrs.user) {
      throw new Error('`team` and `user` on a `crate-ownership` are mutually exclusive');
    }
  },
};
