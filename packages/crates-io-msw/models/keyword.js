import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const schema = z.object({
  id: z.string(),
  keyword: z.string(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'keyword', () => `keyword-${counter}`);
  applyDefault(attrs, 'id', () => attrs.keyword);
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
