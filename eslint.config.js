import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

// The recommended rule sets of ESLint, typescript-eslint and the React hooks
// plugin, unchanged. The one project rule is at the end.
export default tseslint.config(
  {
    ignores: [
      "dist/",
      "target/",
      "node_modules/",
      "crates/",
      "ui/src/bindings.ts",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  reactHooks.configs.flat.recommended,
  {
    rules: {
      // Not a style choice: the frontend must reach Rust only through the
      // generated, typed commands.
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
);
