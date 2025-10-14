import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';
import { dasherize } from '../utils/strings.js';

const schema = z.object({
  id: z.string(),

  category: z.string(),
  slug: z.string(),
  description: z.string(),
  created_at: z.string(),
  crates_cnt: z.number().nullable(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'category', () => `Category ${counter}`);
  applyDefault(attrs, 'slug', () => dasherize(attrs.category));
  applyDefault(attrs, 'id', () => attrs.slug);
  applyDefault(attrs, 'description', () => `This is the description for the category called "${attrs.category}"`);
  applyDefault(attrs, 'created_at', () => '2010-06-16T21:30:45Z');
  applyDefault(attrs, 'crates_cnt', () => null);
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
