import js from "@eslint/js";
import tseslint from "typescript-eslint";

/**
 * ASCII の外の文字（日本語の文字・全角の記号・「※」「→」「×」「①」「—」「…」など）。
 * 画面の文言はここに引っかかる。識別子・API の語彙（`"stay"` など）・CSS は ASCII なので引っかからない。
 * ★ 2026-10-11 レビューで広げた（日本語の範囲だけだと「※」「→」「×」が通った）。
 */
const UI_TEXT = "/[^\\x00-\\x7F]/";

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
  {
    // **画面の文言は `src/messages.ts` にだけ書く**（製造準備 A-2。2026-10-11 本人が「分けて lint で止める」を選んだ）。
    // テスト（`__tests__` / `e2e`）は描かれた文字を突き合わせるので対象外。
    // 許すのは空白だけの JSX の地の文（改行・字下げ）。数字や記号だけの地の文も文言として止める ——
    // 区切りの「・」「—」も言語で変わりうるので、表へ入れる。
    files: ["src/**/*.{ts,tsx}"],
    ignores: ["src/messages.ts", "src/__tests__/**"],
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector: "JSXText[value=/\\S/]",
          message: "画面の文言を JSX に直書きしない。src/messages.ts に足して msg(\"<画面>\") で引く（製造準備 A-2）",
        },
        {
          selector: `Literal[value=${UI_TEXT}]`,
          message: "画面の文言（日本語）を文字列で直書きしない。src/messages.ts に足して msg(\"<画面>\") で引く（製造準備 A-2）",
        },
        {
          selector: `TemplateElement[value.raw=${UI_TEXT}]`,
          message: "画面の文言（日本語）をテンプレート文字列で直書きしない。src/messages.ts に足して msg(\"<画面>\") で引く（製造準備 A-2）",
        },
      ],
    },
  },
);
