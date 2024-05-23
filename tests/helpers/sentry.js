import { MockSentryService } from 'crates-io/utils/sentry';

export function setupSentryMock(hooks) {
  hooks.beforeEach(function () {
    this.owner.register('service:sentry', MockSentryService);
  });
}
