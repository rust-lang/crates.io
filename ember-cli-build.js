'use strict';

const EmberApp = require('ember-cli/lib/broccoli/ember-app');

module.exports = function (defaults) {
  let env = EmberApp.env();
  let isProd = env === 'production';

  let browsers = require('./config/targets').browsers;

  let app = new EmberApp(defaults, {
    '@embroider/macros': {
      setConfig: {
        '@ember-data/store': {
          polyfillUUID: true,
        },
      },
    },

    autoImport: {
      webpack: {
        devtool: isProd ? 'source-map' : 'eval-source-map',
        externals: {
          // prevent Chart.js from bundling Moment.js
          moment: 'moment',
        },
      },
    },
    babel: {
      plugins: [
        require.resolve('ember-auto-import/babel-plugin'),
        require.resolve('ember-concurrency/async-arrow-task-transform'),
        ...require('ember-cli-code-coverage').buildBabelPlugin({ embroider: true }),
      ],
    },

    'ember-cli-babel': {
      throwUnlessParallelizable: true,
    },

    'ember-fetch': {
      preferNative: true,
    },

    cssModules: {
      extension: 'module.css',
      plugins: {
        before: [require('postcss-nested')],
        postprocess: [require('postcss-preset-env')({ browsers, preserve: false })],
      },
    },
    fingerprint: {
      extensions: ['js', 'css', 'png', 'jpg', 'gif', 'map', 'svg', 'ttf', 'woff', 'woff2'],
    },

    sourcemaps: {
      enabled: true,
      extensions: ['js'],
    },
  });

  app.import('node_modules/normalize.css/normalize.css', { prepend: true });
  app.import('vendor/qunit.css', { type: 'test' });

  const { Webpack } = require('@embroider/webpack');
  return require('@embroider/compat').compatBuild(app, Webpack, {
    staticAddonTrees: true,
    staticAddonTestSupportTrees: true,
    staticModifiers: true,
    packagerOptions: {
      webpackConfig: {
        resolve: {
          fallback: {
            // disables `crypto` import warning in `axe-core`
            crypto: false,
            // disables `timers` import warning in `@sinon/fake-timers`
            timers: false,
          },
        },
      },
    },
    packageRules: [
      // see https://github.com/embroider-build/embroider/issues/1322
      {
        package: '@ember-data/store',
        addonModules: {
          '-private.js': {
            dependsOnModules: [],
          },
          '-private/system/core-store.js': {
            dependsOnModules: [],
          },
          '-private/system/model/internal-model.js': {
            dependsOnModules: [],
          },
        },
      },
    ],
  });
};
