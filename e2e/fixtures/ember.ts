/// <reference types="ember-source/types" />

import type Application from '@ember/application';
import type ApplicationInstance from '@ember/application/instance';

import { APP_HOOK_CUSTOM_EVENTS, APP_HOOK_KEY, SENTRY_HOOK_KEY } from '@/app/consts';
import { Page } from '@playwright/test';

const HOOK_MAPPING = {
  hook: APP_HOOK_KEY,
  customEvents: APP_HOOK_CUSTOM_EVENTS,
  sentry: SENTRY_HOOK_KEY,
  owner: 'hook:app:owner',
} as const;

export type EmberPageOptions = {
  setTesting?: boolean;
  mockSentry?: boolean;
};

const DEFAULT_OPTIONS: EmberPageOptions = { setTesting: true, mockSentry: true };

export class EmberPage {
  constructor(public readonly page: Page) {
    this.page = page;
  }

  async addHook(hook: HookFn | HookScript) {
    let fn = String((hookKey: string, hook: HookFn) => {
      let key = Symbol.for(`${hookKey}`);
      window[key] = (window[key] || []).concat(hook);
    });
    await this.page.addInitScript(`(${fn})('${HOOK_MAPPING.hook}', ${hook.toString()});`);
  }

  async evaluate<R>(fn: (owner: ApplicationInstance) => R | Promise<R>) {
    const handle = await this.page.evaluateHandle(async () => {
      let key = Symbol.for(await window.__emberHookMapping('owner'));
      return window[key];
    });
    if (!handle) {
      throw new Error('Ember is not ready.');
    }
    return await handle.evaluate(fn);
  }

  private async addHelpers({ testing = true, mockSentry = true }: { testing: boolean; mockSentry: boolean }) {
    await this.page.exposeFunction('__emberHookMapping', (key: string) => HOOK_MAPPING[key]);
    await this.page.addInitScript(
      ({ event, ownerKey, testing }) => {
        window.addEventListener(
          `${event}`,
          async ({ detail: { owner } }: CustomEvent<{ owner: ApplicationInstance }>) => {
            if (testing) {
              let testingService = owner.lookup('service:testing') as { setTesting: (testing: boolean) => void };
              testingService.setTesting(true);
            }
            window[Symbol.for(`${ownerKey}`)] = owner;
          },
        );
      },
      { event: HOOK_MAPPING.customEvents.initialize, ownerKey: HOOK_MAPPING.owner, testing },
    );
    if (mockSentry) {
      await this.addHook(async owner => {
        let key = await window.__emberHookMapping('sentry');
        owner.register('service:sentry', owner.lookup(key) as object);
      });
    }
  }

  async setup(options: EmberPageOptions = DEFAULT_OPTIONS) {
    const { setTesting: testing, mockSentry } = { ...DEFAULT_OPTIONS, ...options };
    await this.addHelpers({ testing, mockSentry });
  }
}

type HookFn = (owner: ApplicationInstance, app: Application) => void | Promise<void>;
type HookScript = Exclude<Parameters<Page['addInitScript']>[0], Function>;

declare global {
  interface Window {
    __emberHookMapping(s: string): Promise<(typeof HOOK_MAPPING)[keyof typeof HOOK_MAPPING]>;
  }
}
