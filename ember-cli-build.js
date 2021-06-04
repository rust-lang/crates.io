'use strict';

const EmberApp = require('ember-cli/lib/broccoli/ember-app');

const { USE_EMBROIDER } = process.env;

module.exports = function (defaults) {
  let env = EmberApp.env();
  let isProd = env === 'production';

  const highlightedLanguages = [
    'bash',
    'c',
    'clike',
    'glsl',
    'go',
    'ini',
    'javascript',
    'json',
    'markup',
    'protobuf',
    'ruby',
    'rust',
    'scss',
    'sql',
    'toml',
    'yaml',
  ];

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
    'ember-prism': {
      theme: 'twilight',
      components: highlightedLanguages,
    },

    'ember-test-selectors': {
      patchClassicComponent: false,
    },

    cssModules: {
      extension: 'module.css',
      plugins: {
        before: [require('postcss-nested')],
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

  if (USE_EMBROIDER) {
    const { Webpack } = require('@embroider/webpack');
    return require('@embroider/compat').compatBuild(app, Webpack);
  }

  return app.toTree();
};
