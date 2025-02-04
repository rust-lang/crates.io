import { test } from 'vitest';

import { loadFixtures } from './fixtures.js';
import { db } from './index.js';

test('loadFixtures() succeeds', async function () {
  loadFixtures(db);
});
