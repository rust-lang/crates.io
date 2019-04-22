module.exports = {
    root: true,
    parserOptions: {
        ecmaVersion: 2017,
        sourceType: 'module',
        ecmaFeatures: {
            experimentalObjectRestSpread: true,
        },
    },
    plugins: ['ember', 'prettier'],
    extends: ['eslint:recommended', 'plugin:ember/recommended', 'plugin:prettier/recommended'],
    env: {
        browser: true,
    },
    rules: {
        'prettier/prettier': 'error',

        'arrow-parens': 'off',
        'brace-style': 'off',
        camelcase: 'off',
        'comma-dangle': 'off',
        'dot-notation': 'off',
        'operator-linebreak': 'off',
    },
    overrides: [
        // node files
        {
            files: [
                '.eslintrc.js',
                '.template-lintrc.js',
                'ember-cli-build.js',
                'testem.js',
                'blueprints/*/index.js',
                'config/**/*.js',
                'lib/*/index.js',
                'server/**/*.js',
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
