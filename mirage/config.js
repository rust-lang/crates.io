import { createServer } from 'miragejs';

import factories from './factories';
import fixtures from './fixtures';
import models from './models';
import * as RouteHandlers from './route-handlers';
import serializers from './serializers';

export default function makeServer(config) {
  let server = createServer({
    ...config,

    factories,
    fixtures,
    models,
    serializers,

    routes() {
      RouteHandlers.register(this);

      // Used by ember-cli-code-coverage
      this.passthrough('/write-coverage');
    },
    ...getHookConfig(),
  });
  server = processHooks(server);
  return server;
}

export const CONFIG_KEY = 'hook:mirage:config';
export const HOOK_KEY = 'hook:mirage:hook';

// Get injected config for testing with Playwright
function getHookConfig() {
  return window[Symbol.for(CONFIG_KEY)];
}

// Process injected hooks for testing with Playwright
function processHooks(server) {
  let hooks = window[Symbol.for(HOOK_KEY)];
  if (hooks && Array.isArray(hooks)) {
    hooks.forEach(hook => {
      if (hook && typeof hook === 'function') {
        hook(server);
      }
    });
  }
  return server;
}
