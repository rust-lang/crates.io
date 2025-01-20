import { nullable, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';
import { dasherize } from '../utils/strings.js';

export default {
  id: primaryKey(String),

  category: String,
  slug: String,
  description: String,
  created_at: String,
  crates_cnt: nullable(Number),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'category', () => `Category ${counter}`);
    applyDefault(attrs, 'slug', () => dasherize(attrs.category));
    applyDefault(attrs, 'id', () => attrs.slug);
    applyDefault(attrs, 'description', () => `This is the description for the category called "${attrs.category}"`);
    applyDefault(attrs, 'created_at', () => '2010-06-16T21:30:45Z');
    applyDefault(attrs, 'crates_cnt', () => null);
  },
};
