import { nullable, oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

const LICENSES = ['MIT/Apache-2.0', 'MIT', 'Apache-2.0'];

export default {
  id: primaryKey(Number),

  num: String,
  created_at: String,
  updated_at: String,
  yanked: Boolean,
  yank_message: nullable(String),
  license: String,
  downloads: Number,
  features: Object,
  crate_size: Number,
  readme: nullable(String),
  rust_version: nullable(String),

  crate: oneOf('crate'),
  publishedBy: nullable(oneOf('user')),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'num', () => `1.0.${attrs.id - 1}`);
    applyDefault(attrs, 'created_at', () => '2010-06-16T21:30:45Z');
    applyDefault(attrs, 'updated_at', () => '2017-02-24T12:34:56Z');
    applyDefault(attrs, 'yanked', () => false);
    applyDefault(attrs, 'yank_message', () => null);
    applyDefault(attrs, 'license', () => LICENSES[attrs.id % LICENSES.length]);
    applyDefault(attrs, 'downloads', () => (((attrs.id + 13) * 42) % 13) * 1234);
    applyDefault(attrs, 'crate_size', () => (((attrs.id + 13) * 42) % 13) * 54_321);
    applyDefault(attrs, 'readme', () => null);
    applyDefault(attrs, 'rust_version', () => null);

    if (!attrs.crate) {
      throw new Error(`Missing \`crate\` relationship on \`version:${attrs.num}\``);
    }
  },
};
