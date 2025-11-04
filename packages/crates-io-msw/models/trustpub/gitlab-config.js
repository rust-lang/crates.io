import { nullable, oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../../utils/defaults.js';

export default {
  id: primaryKey(Number),

  crate: oneOf('crate'),
  namespace: String,
  namespace_id: nullable(String),
  project: String,
  workflow_filepath: String,
  environment: nullable(String),
  created_at: String,

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'namespace', () => 'rust-lang');
    applyDefault(attrs, 'namespace_id', () => null);
    applyDefault(attrs, 'project', () => `repo-${attrs.id}`);
    applyDefault(attrs, 'workflow_filepath', () => '.gitlab-ci.yml');
    applyDefault(attrs, 'environment', () => null);
    applyDefault(attrs, 'created_at', () => '2023-01-01T00:00:00Z');
  },
};
