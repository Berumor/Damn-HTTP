import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "dist/",
      "target/",
      "node_modules/",
      "crates/",
      "ui/src/bindings.ts",
      "eslint.config.js",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: {
      "react-hooks": reactHooks,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tauri-apps/api/core",
              message:
                "Call Rust through the generated commands in ui/src/bindings.ts, not invoke() with a string name.",
            },
          ],
        },
      ],
    },
  },
  {
    // Mocks are passed around as plain functions in tests.
    files: ["ui/src/**/*.test.{ts,tsx}", "ui/src/test/**"],
    rules: {
      "@typescript-eslint/unbound-method": "off",
    },
  },
);
