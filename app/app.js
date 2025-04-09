import Application from '@ember/application';

import { importSync, isDevelopingApp, macroCondition } from '@embroider/macros';
import loadInitializers from 'ember-load-initializers';
import Resolver from 'ember-resolver';

import config from 'crates-io/config/environment';

import * as Sentry from './sentry';

// eslint-disable-next-line unicorn/prefer-add-event-listener
window.onerror = undefined;
Sentry.init();
if (macroCondition(isDevelopingApp())) {
  importSync('./deprecation-workflow');
}

export default class App extends Application {
  modulePrefix = config.modulePrefix;
  podModulePrefix = config.podModulePrefix;
  Resolver = Resolver;

  customEvents = {
    // prevent non-passive listeners for touchstart/touchmove events
    touchstart: null,
    touchmove: null,
  };
}

loadInitializers(App, config.modulePrefix);
