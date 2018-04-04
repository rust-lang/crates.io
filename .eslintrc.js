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
        'ember/use-brace-expansion': 'off',
        'ember/no-on-calls-in-components': 'off',
        'ember/no-capital-letters-in-routes': 'off',
        'ember/new-module-imports': 'off',
    },
};
