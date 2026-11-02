// Copied beside the pinned packages under target/qa/tooling/lint before each run.
import js from '@eslint/js';
import globals from 'globals';

export default [
    {
        ignores: [
            '**/vendor/**',
            '**/pkg/**',
            '**/*jsQR*.js',
            '**/node_modules/**',
            '**/build/**',
            'external/**',
            'target/**',
        ],
    },
    js.configs.recommended,
    {
        files: ['**/*.js', '**/*.mjs'],
        languageOptions: {
            ecmaVersion: 'latest',
            sourceType: 'module',
            globals: { ...globals.browser, ...globals.node, jsQR: 'readonly' },
        },
        linterOptions: { reportUnusedDisableDirectives: 'error' },
        rules: {
            complexity: ['error', 12],
            'no-unused-vars': ['error', { argsIgnorePattern: '^_', destructuredArrayIgnorePattern: '^_' }],
        },
    },
];
