module.exports = {
    root: true,
    parserOptions: {
        ecmaVersion: 2017,
        sourceType: 'module',
        ecmaFeatures: {
            'experimentalObjectRestSpread': true,
        },
    },
    extends: [
        'eslint:recommended',
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
    },
};
