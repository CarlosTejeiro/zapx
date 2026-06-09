import globals from 'globals'
import tseslint from 'typescript-eslint'
import svelte from 'eslint-plugin-svelte'

export default [
  ...tseslint.configs.recommended,
  ...svelte.configs['flat/recommended'],

  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.es2022 },
    },
  },

  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },

  {
    rules: {
      // Enforce the style guide: unknown over any
      '@typescript-eslint/no-explicit-any': 'error',
    },
  },

  {
    ignores: ['dist/', 'node_modules/', '.svelte-kit/'],
  },
]
