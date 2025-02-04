import { nullable, oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../utils/defaults.js';

const REQS = ['^0.1.0', '^2.1.3', '0.3.7', '~5.2.12'];

export default {
  id: primaryKey(Number),

  default_features: Boolean,
  features: Array,
  kind: String,
  optional: Boolean,
  req: String,
  target: nullable(String),

  crate: oneOf('crate'),
  version: oneOf('version'),

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'default_features', () => counter % 4 === 3);
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
  },
};
