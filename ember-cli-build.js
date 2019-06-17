'use strict';

const EmberApp = require('ember-cli/lib/broccoli/ember-app');

module.exports = function(defaults) {
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
        sassOptions: {
            includePaths: ['node_modules/normalize.css'],
        },
        fingerprint: {
            extensions: ['js', 'css', 'png', 'jpg', 'gif', 'map', 'svg', 'ttf', 'woff', 'woff2'],
        },
    });

    // Use `app.import` to add additional libraries to the generated
    // output files.
    //
    // If you need to use different assets in different
    // environments, specify an object as the first parameter. That
    // object's keys should be the environment name and the values
    // should be the asset to use in that environment.
    //
    // If the library that you are including contains AMD or ES6
    // modules that you would like to import into your application
    // please specify an object with the list of modules as keys
    // along with the exports of each module as its value.
    app.import('node_modules/timekeeper/lib/timekeeper.js', {
        using: [{ transformation: 'cjs', as: 'timekeeper' }],
    });

    return app.toTree();
};
