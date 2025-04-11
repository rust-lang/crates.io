'use strict';

const EmberApp = require('ember-cli/lib/broccoli/ember-app');

module.exports = function (defaults) {
  let env = EmberApp.env();
  let isProd = env === 'production';

  let extraPublicTrees = [];
  if (!isProd) {
    const path = require('node:path');
    const funnel = require('broccoli-funnel');

    let mswPath = require.resolve('msw/mockServiceWorker.js');
    let mswParentPath = path.dirname(mswPath);

    extraPublicTrees.push(funnel(mswParentPath, { include: ['mockServiceWorker.js'] }));
  }

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

    cssModules: {
      extension: 'module.css',
      // see https://github.com/salsify/ember-css-modules/blob/v2.0.1/docs/ORDERING.md
      headerModules: [
        'crates-io/styles/shared/a11y',
        'crates-io/styles/shared/buttons',
        'crates-io/styles/shared/forms',
        'crates-io/styles/shared/sort-by',
        'crates-io/styles/shared/typography',
        'crates-io/styles/application',
        // for the `.scopes-list` class
        'crates-io/styles/settings/tokens/new',
        // for the `.box-link` class
        'crates-io/components/front-page-list/item',
      ],
      plugins: {
        postprocess: [
          require('postcss-preset-env')({
            browsers,
            preserve: false,
            features: {
              'nesting-rules': true,
            },
          }),
        ],
      },
    },

    emberData: {
      deprecations: {
        DEPRECATE_STORE_EXTENDS_EMBER_OBJECT: false,
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
    extraPublicTrees,
    staticAddonTrees: true,
    staticAddonTestSupportTrees: true,
    staticModifiers: true,
    packagerOptions: {
      webpackConfig: {
        externals: ({ request, context }, callback) => {
          // Prevent `@mswjs/data` from bundling the `msw` package.
          //
          // `@crates-io/msw` is importing the ESM build of the `msw` package, but
          // `@mswjs/data` is trying to import the CJS build instead. This is causing
          // a conflict within webpack. Since we don't need the functionality within
          // `@mswjs/data` that requires the `msw` package, we can safely ignore this
          // import.
          if (request == 'msw' && context.includes('@mswjs/data')) {
            return callback(null, request, 'global');
          }
          callback();
        },
        resolve: {
          fallback: {
            // disables `crypto` import warning in `axe-core`
            crypto: false,
            // disables `timers` import warning in `@sinon/fake-timers`
            timers: false,
            // disables `util` import warning in `@sinon/fake-timers`
            util: false,
          },
        },
      },
    },
  });
};
