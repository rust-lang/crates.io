import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const schema = v.object({
  id: v.string(),
  keyword: v.string(),
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
