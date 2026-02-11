import { setupWorker } from 'msw/browser';
import { test as testBase } from 'vitest';

let worker = setupWorker();

export let test = testBase.extend<{ worker: ReturnType<typeof setupWorker> }>({
  worker: [
    // eslint-disable-next-line no-empty-pattern
    async ({}, use) => {
      await worker.start({ quiet: true, onUnhandledRequest: 'error' });
      await use(worker);
      worker.resetHandlers();
      worker.stop();
    },
    { auto: true },
  ],
});
