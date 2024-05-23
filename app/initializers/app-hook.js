import config from 'crates-io/config/environment';
import { APP_HOOK_CUSTOM_EVENTS, APP_HOOK_KEY, SENTRY_HOOK_KEY } from 'crates-io/consts';
import { MockSentryService } from 'crates-io/utils/sentry';

export function initialize(app) {
  if (config.environment === 'production') {
    return;
  }

  // This is not available in Ember test
  const owner = app.__container__?.owner;
  if (!owner) {
    return;
  }

  // Pre-register MockSentryService to make it accessible through owner.lookup.
  // This allows mocking it as a real service in Playwright.
  app.register(SENTRY_HOOK_KEY, MockSentryService, { instantiate: false });

  // Use deferReadiness and advanceReadiness to guarantee all hooks are processed before the app is
  // ready.
  app.deferReadiness();
  processHooks(owner, app).then(() => app.advanceReadiness());

  window.dispatchEvent(new CustomEvent(APP_HOOK_CUSTOM_EVENTS.initialize, { detail: { owner } }));
}

async function processHooks(owner, app) {
  const hooks = window[Symbol.for(APP_HOOK_KEY)];
  if (hooks && Array.isArray(hooks)) {
    for (let hook of hooks) {
      await hook(owner, app);
    }
  }
}

export default {
  name: 'app.hook',
  initialize,
};
