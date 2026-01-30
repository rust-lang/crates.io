'use strict';

const EmberApp = require('ember-cli/lib/broccoli/ember-app');

module.exports = function (defaults) {
  let env = EmberApp.env();
  let isProd = env === 'production';

  let extraPublicTrees = [];
  if (!isProd) {
    let path = require('node:path');
    let funnel = require('broccoli-funnel');

    let mswPath = require.resolve('msw/mockServiceWorker.js');
    let mswParentPath = path.dirname(mswPath);

    extraPublicTrees.push(funnel(mswParentPath, { include: ['mockServiceWorker.js'] }));
  }

  let app = new EmberApp(defaults, {
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
      ],
    },

    'ember-cli-babel': {
      throwUnlessParallelizable: true,
    },

    emberData: {
      polyfillUUID: true,

      deprecations: {
        DEPRECATE_STORE_EXTENDS_EMBER_OBJECT: false,
      },
    },

    fingerprint: {
      extensions: ['js', 'css', 'png', 'jpg', 'gif', 'map', 'svg', 'ttf', 'woff', 'woff2', 'wasm'],
    },

    sourcemaps: {
      enabled: true,
      extensions: ['js'],
    },
  });

  // app.import('node_modules/normalize.css/normalize.css');
  app.import('vendor/qunit.css', { type: 'test' });

  let { Webpack } = require('@embroider/webpack');
  return require('@embroider/compat').compatBuild(app, Webpack, {
    extraPublicTrees,
    staticAddonTrees: true,
    staticAddonTestSupportTrees: true,
    staticModifiers: true,
    packagerOptions: {
      webpackConfig: {
        experiments: {
          asyncWebAssembly: true,
        },
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
        module: {
          rules: [
            // CSS loaders for scoped CSS
            {
              test: /\.css$/,
              use: [
                { loader: require.resolve('ember-scoped-css/build/app-css-loader') },
                {
                  loader: 'postcss-loader',
                  options: {
                    postcssOptions: {
                      plugins: [['postcss-preset-env', { features: { 'nesting-rules': true } }]],
                    },
                  },
                },
              ],
            },
            {
              test: /\.svg$/,
              type: 'asset/resource',
            },
          ],
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
