import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const schema = v.object({
  // `v.string()` is used to support some of our old fixtures that use strings here for some reason
  id: v.union([v.number(), v.string()]),

  name: v.string(),
  description: v.string(),
  downloads: v.number(),
  recent_downloads: v.number(),
  documentation: v.nullable(v.string()),
  homepage: v.nullable(v.string()),
  repository: v.nullable(v.string()),
  created_at: v.string(),
  updated_at: v.string(),
  badges: v.array(v.any()),
  _extra_downloads: v.array(v.any()),
  trustpubOnly: v.boolean(),

  categories: v.optional(v.array(v.any()), () => []),
  keywords: v.optional(v.array(v.any()), () => []),
});

function preCreate(attrs, counter) {
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
  applyDefault(attrs, 'badges', () => []);
  applyDefault(attrs, '_extra_downloads', () => []);
  applyDefault(attrs, 'trustpubOnly', () => false);
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
