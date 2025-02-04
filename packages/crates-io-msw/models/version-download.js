import { oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

export default {
  id: primaryKey(Number),

  date: String,
  downloads: Number,

  version: oneOf('version'),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'date', () => '2019-05-21');
    applyDefault(attrs, 'downloads', () => (((attrs.id + 13) * 42) % 13) * 2345);

    if (!attrs.version) {
      throw new Error('Missing `version` relationship on `version-download`');
    }
  },
};
