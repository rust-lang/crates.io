import { manyOf, nullable, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

export default {
  id: primaryKey(Number),

  name: String,
  description: String,
  downloads: Number,
  recent_downloads: Number,
  documentation: nullable(String),
  homepage: nullable(String),
  repository: nullable(String),
  created_at: String,
  updated_at: String,
  badges: Array,
  _extra_downloads: Array,

  categories: manyOf('category'),
  keywords: manyOf('keyword'),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'name', () => `crate-${attrs.id}`);
    applyDefault(attrs, 'description', () => `This is the description for the crate called "${attrs.name}"`);
    applyDefault(attrs, 'downloads', () => (((attrs.id + 13) * 42) % 13) * 12_345);
    applyDefault(attrs, 'recent_downloads', () => (((attrs.id + 7) * 31) % 13) * 321);
    applyDefault(attrs, 'documentation', () => null);
    applyDefault(attrs, 'homepage', () => null);
    applyDefault(attrs, 'repository', () => null);
    applyDefault(attrs, 'created_at', () => '2010-06-16T21:30:45Z');
    applyDefault(attrs, 'updated_at', () => '2017-02-24T12:34:56Z');
  },
};
