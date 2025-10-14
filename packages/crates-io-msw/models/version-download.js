import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const schema = z.object({
  id: z.number(),

  date: z.string(),
  downloads: z.number(),

  version: z.any(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);
  applyDefault(attrs, 'date', () => '2019-05-21');
  applyDefault(attrs, 'downloads', () => (((attrs.id + 13) * 42) % 13) * 2345);

  if (!attrs.version) {
    throw new Error('Missing `version` relationship on `version-download`');
  }
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
