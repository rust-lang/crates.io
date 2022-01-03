import FakeTimers from '@sinonjs/fake-timers';

export function setupFakeTimers(hooks, now = '2017-11-20T12:00:00') {
  hooks.beforeEach(function () {
    this.clock = FakeTimers.install({
      now: Date.parse(now),
      toFake: ['Date'],
      shouldAdvanceTime: true,
    });
  });

  hooks.afterEach(function () {
    this.clock?.uninstall();
  });
}
