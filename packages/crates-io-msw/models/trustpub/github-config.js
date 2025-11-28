import { Collection } from '@msw/data';
import { z } from 'zod';

import { applyDefault } from '../../utils/defaults.js';
import { preCreateExtension } from '../../utils/pre-create-extension.js';

const schema = z.object({
  id: z.number(),

  crate: z.any().nullable(),
  repository_owner: z.string(),
  repository_owner_id: z.number(),
  repository_name: z.string(),
  workflow_filename: z.string(),
  environment: z.string().nullable(),
  created_at: z.string(),
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
