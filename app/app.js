import Application from '@ember/application';

import loadInitializers from 'ember-load-initializers';
import Resolver from 'ember-resolver';

import config from './config/environment';
import * as Sentry from './sentry';

import './app.css';

import { importSync, isDevelopingApp, macroCondition } from '@embroider/macros';

// Will only import your deprecation-workflow file while isDevelopingApp is true
// and will be stripped from your build if isDevelopingApp is false
//
// If you want to always include deprecation-workflow,
// use:
//   import './deprecation-workflow.js';
if (macroCondition(isDevelopingApp())) {
  importSync('./deprecation-workflow.js');
}

// eslint-disable-next-line unicorn/prefer-add-event-listener
window.onerror = undefined;
Sentry.init();

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
