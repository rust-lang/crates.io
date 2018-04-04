module.exports = {
    root: true,
    parserOptions: {
        ecmaVersion: 2017,
        sourceType: 'module',
        ecmaFeatures: {
            'experimentalObjectRestSpread': true,
        },
    },
    plugins: [
        'ember',
    ],
    extends: [
        'eslint:recommended',
        'plugin:ember/recommended',
    ],
    env: {
        browser: true,
    },
    rules: {
        'arrow-parens': 'off',
        'brace-style': 'off',
        'camelcase': 'off',
        'comma-dangle': 'off',
        'dot-notation': 'off',
        'indent': ['error', 4],
        'operator-linebreak': 'off',
        'quotes': ['error', 'single', {
            'allowTemplateLiterals': true,
            'avoidEscape': true,
        }],
        'ember/no-on-calls-in-components': 'off',
    },
    overrides: [
        // node files
        {
            files: [
                'testem.js',
                'ember-cli-build.js',
                'config/**/*.js',
                'lib/*/index.js',
            ],
            parserOptions: {
                sourceType: 'script',
                ecmaVersion: 2015,
            },
            env: {
                browser: false,
                node: true,
            },
        },
    ],
};
