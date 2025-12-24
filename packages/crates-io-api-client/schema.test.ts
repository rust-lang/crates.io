import * as fs from 'node:fs/promises';
import * as path from 'node:path';

import openapiTS, { astToString } from 'openapi-typescript';
import { expect, test } from 'vitest';

const SNAPSHOT_PATH = '../../src/tests/snapshots/integration__openapi__openapi_snapshot-2.snap';

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
  let ast = await openapiTS(schema);
  return HEADER + astToString(ast);
}

test('schema.d.ts is up to date', async () => {
  let generated = await generateSchema();
  let schemaPath = path.resolve(__dirname, 'schema.d.ts');
  await expect(generated).toMatchFileSnapshot(schemaPath);
});
