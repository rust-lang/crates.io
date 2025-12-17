import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../../utils/defaults.js';
import { preCreateExtension } from '../../utils/pre-create-extension.js';

const schema = v.object({
  id: v.number(),

  crate: v.any(),
  repository_owner: v.string(),
  repository_owner_id: v.number(),
  repository_name: v.string(),
  workflow_filename: v.string(),
  environment: v.nullable(v.string()),
  created_at: v.string(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);
  applyDefault(attrs, 'repository_owner', () => 'rust-lang');
  applyDefault(attrs, 'repository_owner_id', () => 5_430_905);
  applyDefault(attrs, 'repository_name', () => `repo-${attrs.id}`);
  applyDefault(attrs, 'workflow_filename', () => 'ci.yml');
  applyDefault(attrs, 'environment', () => null);
  applyDefault(attrs, 'created_at', () => '2023-01-01T00:00:00Z');
  applyDefault(attrs, 'crate', () => null);
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
