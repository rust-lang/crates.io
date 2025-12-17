import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';
import { dasherize } from '../utils/strings.js';

const schema = v.object({
  id: v.string(),

  category: v.string(),
  slug: v.string(),
  description: v.string(),
  created_at: v.string(),
  crates_cnt: v.nullable(v.number()),
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
