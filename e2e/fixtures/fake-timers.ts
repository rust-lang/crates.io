import { Page } from '@playwright/test';
import { withGlobal, FakeTimerWithContext, FakeTimerInstallOpts } from '@sinonjs/fake-timers';

export type FakeTimersOptions = Omit<FakeTimerInstallOpts, 'now'> & { now?: FakeTimerInstallOpts['now'] | string };

export class FakeTimers {
  constructor(public readonly page: Page) {
    this.page = page;
  }

  async setup(options: FakeTimersOptions = {}) {
    let fn = String(
      (
        withGlobal: FakeTimerWithContext['withGlobal'],
        options: FakeTimerInstallOpts = {},
        context: typeof globalThis = globalThis,
      ) => {
        context['globalObject'] = undefined;
        context['clock']?.uninstall?.();
        let opts = { ...options };
        if (typeof opts.now === 'string') {
          opts.now = Date.parse(opts.now);
        }
        context['clock'] = withGlobal(context).install(opts);
      },
    );
    await this.page.addInitScript(`(${fn})(${String(withGlobal)}, ${JSON.stringify(options)});`);
  }
}
