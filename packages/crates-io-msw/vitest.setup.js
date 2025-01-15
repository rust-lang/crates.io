import { drop } from '@mswjs/data';
import { setupServer } from 'msw/node';
import { afterAll, afterEach, beforeAll } from 'vitest';

import { db, handlers } from './index.js';

const server = setupServer(...handlers);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterEach(() => drop(db));
afterEach(() => {
  Object.values(db).forEach(model => {
    if (model.counter) {
      model.counter = 0;
    }
  });
});
afterAll(() => server.close());
