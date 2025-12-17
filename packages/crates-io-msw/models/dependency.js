import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../utils/defaults.js';
import { preCreateExtension } from '../utils/pre-create-extension.js';

const REQS = ['^0.1.0', '^2.1.3', '0.3.7', '~5.2.12'];

const schema = v.object({
  id: v.number(),

  default_features: v.boolean(),
  features: v.array(v.any()),
  kind: v.string(),
  optional: v.boolean(),
  req: v.string(),
  target: v.nullable(v.string()),

  crate: v.any(),
  version: v.any(),
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
