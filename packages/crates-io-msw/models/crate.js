import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const schema = z.object({
  id: z.number(),

  name: z.string(),
  description: z.string(),
  downloads: z.number(),
  recent_downloads: z.number(),
  documentation: z.string().nullable(),
  homepage: z.string().nullable(),
  repository: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
  badges: z.array(z.any()),
  _extra_downloads: z.array(z.any()),
  trustpubOnly: z.boolean(),

  categories: z.array(z.any()).default(() => []),
  keywords: z.array(z.any()).default(() => []),
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
