import { setupWorker } from 'msw/browser';
import { test as testBase } from 'vitest';

type Worker = ReturnType<typeof setupWorker>;

export let test = testBase.extend<{ _mswWorker: Worker; worker: Worker }>({
  _mswWorker: [
    // eslint-disable-next-line no-empty-pattern
    async ({}, use) => {
      let worker = setupWorker();
      await worker.start({ quiet: true, onUnhandledRequest: 'error' });
      await use(worker);
      worker.stop();
    },
    { scope: 'worker' },
  ],
  worker: [
    async ({ _mswWorker }, use) => {
      await use(_mswWorker);
      _mswWorker.resetHandlers();
    },
    { auto: true },
  ],
});
