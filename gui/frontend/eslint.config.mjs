import { FlatCompat } from '@eslint/eslintrc';

// https://nextjs.org/docs/app/api-reference/config/eslint#disabling-rules

const compat = new FlatCompat({
  // import.meta.dirname is available after Node.js v20.11.0
  baseDirectory: import.meta.dirname,
});

const eslintConfig = [
  ...compat.config({
    extends: ['next/core-web-vitals', 'plugin:@next/next/recommended'],
    settings: {
      next: {
        rootDir: 'gui/frontend/',
      },
    },

    rules: {
      'import/order': [
        'warn',
        {
          alphabetize: {
            caseInsensitive: true,
            order: 'asc',
          },
          groups: ['builtin', 'external', 'internal', 'parent', 'sibling', 'index', 'object', 'type'],
          'newlines-between': 'always',
          pathGroupsExcludedImportTypes: ['builtin'],
          pathGroups: [
            { pattern: '@/**', group: 'internal', position: 'before' },
            { pattern: '@/**.css', group: 'index', position: 'before' },
            { pattern: '@/**.json', group: 'index', position: 'before' },
          ],
        },
      ],

      'react/jsx-sort-props': 'warn',
    },
  }),
];

export default eslintConfig;
