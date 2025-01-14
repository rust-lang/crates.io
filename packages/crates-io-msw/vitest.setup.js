import { drop } from '@mswjs/data';
import { setupServer } from 'msw/node';
import { afterAll, afterEach, beforeAll } from 'vitest';

import { db, handlers } from './index.js';

const server = setupServer(...handlers);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterEach(() => drop(db));
afterAll(() => server.close());
