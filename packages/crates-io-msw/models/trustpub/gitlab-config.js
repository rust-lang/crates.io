import { Collection } from '@msw/data';
import * as v from 'valibot';

import { applyDefault } from '../../utils/defaults.js';
import { preCreateExtension } from '../../utils/pre-create-extension.js';

const schema = v.object({
  id: v.number(),

  crate: v.any(),
  namespace: v.string(),
  namespace_id: v.nullable(v.string()),
  project: v.string(),
  workflow_filepath: v.string(),
  environment: v.nullable(v.string()),
  created_at: v.string(),
});

function preCreate(attrs, counter) {
  applyDefault(attrs, 'id', () => counter);
  applyDefault(attrs, 'namespace', () => 'rust-lang');
  applyDefault(attrs, 'namespace_id', () => null);
  applyDefault(attrs, 'project', () => `repo-${attrs.id}`);
  applyDefault(attrs, 'workflow_filepath', () => '.gitlab-ci.yml');
  applyDefault(attrs, 'environment', () => null);
  applyDefault(attrs, 'created_at', () => '2023-01-01T00:00:00Z');
  applyDefault(attrs, 'crate', () => null);
}

const collection = new Collection({
  schema,
  extensions: [preCreateExtension(preCreate)],
});

export default collection;
