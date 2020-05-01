'use strict';

const EmberApp = require('ember-cli/lib/broccoli/ember-app');

module.exports = function (defaults) {
  const highlightedLanguages = [
    'bash',
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
    'yaml',
  ];

  let app = new EmberApp(defaults, {
    babel6: {
      plugins: ['transform-object-rest-spread'],
    },
    'ember-prism': {
      theme: 'twilight',
      components: highlightedLanguages,
    },
    cssModules: {
      extension: 'module.css',
      plugins: {
        before: [require('postcss-nested')],
      },
    },
    'ember-cli-page-progress': {
      includeCss: false,
    },
    fingerprint: {
      extensions: ['js', 'css', 'png', 'jpg', 'gif', 'map', 'svg', 'ttf', 'woff', 'woff2'],
    },
  });

  app.import('node_modules/normalize.css/normalize.css');

  return app.toTree();
};
