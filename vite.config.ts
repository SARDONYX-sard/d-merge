import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: { options: { typeAware: true, typeCheck: true } },
  // NOTE: https://github.com/oxc-project/oxc-vscode/issues/215#issuecomment-4283558960
  fmt: {
    sortImports: {
      newlinesBetween: false,
      groups: [
        ['value-builtin', 'value-external'],
        ['value-internal', 'value-parent', 'value-sibling', 'value-index'],
        { newlinesBetween: true },
        'type-import',
        'unknown',
      ],
    },
    jsxSingleQuote: true,
    printWidth: 120,
    semi: true,
    singleQuote: true,
    sortPackageJson: true,
    ignorePatterns: ['routeTree.gen.ts', 'cspell.jsonc', 'CHANGELOG.md'],
  },
});
