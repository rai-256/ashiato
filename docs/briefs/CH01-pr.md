# CH01 上流: ST17 の /code-review の 9 件を直す

ST17（毎日の気分。archive 済み）に組み込みの `/code-review` を当てて出た 9 件（依頼は `docs/changes/CH01.md`）を、直すか、直さない理由を残すかで閉じる change の上流。
change: `openspec/changes/ch01-st17-review-fixes/`

## 深掘りの答え（第 1 回。推奨で先に書いた版）

第 1 回は推奨の答えで先に仕様と GO 資料を書き、問いと 1 枚で渡す。Q1〜Q7 はすべて B（失われるものは無い）。本人の答えが推奨と違えば、その Q だけ書き直す。

| Q | 推奨 | 写った先 |
|---|---|---|
| Q1 錠の節の置き場 | `*.d/` を読む口を戻しの前へ動かし、ST17 の節を自立した 1 本にして移す | D1（仮）・Task 2 |
| Q2 途中への節の書き足し | `check-shared-appends.sh` が本体の途中の新しい節の見出しも落とす | D2（仮）・Task 1 |
| Q3 終わりの読めない滞在（指摘 4） | 直さない。コメントを正確にし、ST23 へ申し送る | D9（仮）・Task 5 |
| Q4 滞在の行の押した印（指摘 5） | 断られたら最新の記録の値に戻す。届かなかったら残す。日の欄は spec どおり | D5（仮）・spec・Task 7 |
| Q5 開いたまま日をまたぐ（指摘 6） | 押したときに決め直し、「振り返りとして残しました」と知らせる | D6（仮）・spec・Task 8 |
| Q6 同じ日を 2 回読む（指摘 8） | 30 日の 1 回にまとめる | D8（仮）・spec・Task 6 |
| Q7 人が書いた記録の規則の写し（指摘 7） | 1 つの関数にまとめる。種別と順は変えない | D7（仮）・personal-entities の spec・Task 4 |

問わずに決めたもの: 指摘 2（NG の後に OK を出さない。D3）・指摘 3（トークンを追跡されない `local.properties` から渡す。D4）・指摘 9（既存の索引に乗るので直さない。D10）。

## 仮決め

D1・D2・D5・D6・D7・D8・D9（すべて `design.md` に反転条件つき）。

## capability

- delta: `subjective-log`（画面の 3 要件を MODIFIED）・`personal-entities`（主張と場所の拒む 2 要件を MODIFIED。いまの拒む順と地域のずれの範囲を固定する）
- `docs/changes/CH01.md` の `capability` に `personal-entities` を足した（Q7 の帰結）。前倒しは無い。`browsing-views` は delta 無し
- 走っている Story は無い

## tasks

9 Task（共有の本体の検査 / 錠の節の移し替え / 録画のトークン / 人が書いた記録の規則 / 直さない 2 件の理由 / 読み出し 1 回 / 滞在の行の印 / 日をまたぐ前後 / 全体の検査）。画面の Scenario は `web/e2e/feel-ch01.spec.ts`。人間の確認待ちは無い。

## レビュー

- `review/deep.md`: 7 件（fixed 4 / escalated 3 —— 問いの分類と前提を直して渡した）
- `review/spec.md`: 20 件、すべて fixed（D 番号 8 件は仮、ほかは spec・tasks・proposal の直し）。分類を直したもの 3 件（R11・R19 は daily → technical、R14 は premise → conflict。理由は各項）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
