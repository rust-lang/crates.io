import { setupServer } from 'msw/node';
import { afterAll, afterEach, beforeAll } from 'vitest';

import { db, handlers } from './index.js';

// Polyfill `location.href` for MSW to resolve relative URLs
globalThis.location = { href: 'https://crates.io/' };

const server = setupServer(...handlers);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterEach(() => db.reset());
afterAll(() => server.close());
