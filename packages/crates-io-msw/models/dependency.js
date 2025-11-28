import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const REQS = ['^0.1.0', '^2.1.3', '0.3.7', '~5.2.12'];

const schema = z.object({
  id: z.number(),

  default_features: z.boolean(),
  features: z.array(z.any()),
  kind: z.string(),
  optional: z.boolean(),
  req: z.string(),
  target: z.string().nullable(),

  crate: z.any(),
  version: z.any(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);
  applyDefault(attrs, 'default_features', () => counter % 4 === 3);
  applyDefault(attrs, 'features', () => []);
  applyDefault(attrs, 'kind', () => (counter % 3 === 0 ? 'dev' : 'normal'));
  applyDefault(attrs, 'optional', () => counter % 4 !== 3);
  applyDefault(attrs, 'req', () => REQS[counter % REQS.length]);
  applyDefault(attrs, 'target', () => null);

  if (!attrs.crate) {
    throw new Error(`Missing \`crate\` relationship on \`dependency:${attrs.id}\``);
  }
  if (!attrs.version) {
    throw new Error(`Missing \`version\` relationship on \`dependency:${attrs.id}\``);
  }
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
