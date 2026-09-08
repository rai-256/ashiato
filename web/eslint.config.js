import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    rules: {
      // 既定より一段厳しくする（製造準備 C）。フレームワーク既定のままにしない。
      eqeqeq: "error",
      "no-console": ["error", { allow: ["error"] }],
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/explicit-module-boundary-types": "error",
    },
  },
);
