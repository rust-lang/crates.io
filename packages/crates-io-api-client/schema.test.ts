import type { components } from './schema';

import * as fs from 'node:fs/promises';
import path from 'node:path';

import openapiTS, { astToString } from 'openapi-typescript';
import { expect, expectTypeOf, test } from 'vitest';

const SNAPSHOT_PATH = '../../src/tests/snapshots/integration__openapi__openapi_internal_snapshot-2.snap';

type TrustpubData = NonNullable<components['schemas']['Version']['trustpub_data']>;

const HEADER = `/**
 * This file is auto-generated. Do not edit manually.
 *
 * Run \`pnpm --filter @crates-io/api-client regenerate\` to update this file.
 */

`;

async function generateSchema() {
  // Read snapshot file
  let content = await fs.readFile(path.resolve(__dirname, SNAPSHOT_PATH), 'utf8');

  // Strip YAML frontmatter (everything before the first `{`)
  let jsonStart = content.indexOf('{');
  let json = content.slice(jsonStart);

  // Parse and generate TypeScript
  let schema = JSON.parse(json);
  let ast = await openapiTS(schema, { enumValues: true });
  return HEADER + astToString(ast);
}

test('schema.ts is up to date', async () => {
  let generated = await generateSchema();
  let schemaPath = path.resolve(__dirname, 'schema.ts');
  await expect(generated).toMatchFileSnapshot(schemaPath);
});

test('trusted publishing data preserves provider-specific fields', () => {
  type GitHubTrustpubData = Extract<TrustpubData, { provider: 'github' }>;
  type GitLabTrustpubData = Extract<TrustpubData, { provider: 'gitlab' }>;

  expectTypeOf<GitHubTrustpubData>().toMatchObjectType<{
    repository: string;
    run_id: string;
    sha: string;
  }>();
  expectTypeOf<GitLabTrustpubData>().toMatchObjectType<{
    project_path: string;
    job_id: string;
    sha: string;
  }>();
});
