'use strict';

const EmberApp = require('ember-cli/lib/broccoli/ember-app');
const postcssCustomMedia = require('postcss-custom-media');

module.exports = function (defaults) {
  let env = EmberApp.env();
  let isProd = env === 'production';

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
      plugins: [require.resolve('ember-auto-import/babel-plugin')],
    },
    'ember-fetch': {
      preferNative: true,
    },

    cssModules: {
      extension: 'module.css',
      plugins: {
        before: [require('postcss-nested')],
        after: [
          postcssCustomMedia({
            importFrom: `${__dirname}/app/styles/breakpoints.css`,
          }),
        ],
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
    staticAddonTestSupportTrees: true,
    staticModifiers: true,
    packagerOptions: {
      webpackConfig: {
        resolve: {
          // disables `crypto` import warning in `axe-core`
          fallback: { crypto: false },
        },
      },
    },
  });
};
