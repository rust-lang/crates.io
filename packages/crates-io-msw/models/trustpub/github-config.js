import { nullable, oneOf, primaryKey } from '@mswjs/data';

import { applyDefault } from '../../utils/defaults.js';

export default {
  id: primaryKey(Number),

  crate: oneOf('crate'),
  repository_owner: String,
  repository_owner_id: Number,
  repository_name: String,
  workflow_filename: String,
  environment: nullable(String),
  created_at: String,

  preCreate(attrs, counter) {
    applyDefault(attrs, 'id', () => counter);
    applyDefault(attrs, 'repository_owner', () => 'rust-lang');
    applyDefault(attrs, 'repository_owner_id', () => 5_430_905);
    applyDefault(attrs, 'repository_name', () => `repo-${attrs.id}`);
    applyDefault(attrs, 'workflow_filename', () => 'ci.yml');
    applyDefault(attrs, 'environment', () => null);
    applyDefault(attrs, 'created_at', () => '2023-01-01T00:00:00Z');
  },
};
