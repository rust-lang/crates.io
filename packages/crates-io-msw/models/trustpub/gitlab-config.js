import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../../utils/defaults.js';
import { preCreateExtension } from '../../utils/pre-create-extension.js';

const schema = z.object({
  id: z.number(),

  crate: z.any().nullable(),
  namespace: z.string(),
  namespace_id: z.string().nullable(),
  project: z.string(),
  workflow_filepath: z.string(),
  environment: z.string().nullable(),
  created_at: z.string(),
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
