# ST48 古い版が新しい DB を書き換えない

change: `openspec/changes/st48-db-version-guard/`。新しい capability `distribution`（ADDED 7・Scenario 41 本）。満たす要件は FR-117・FR-118・FR-129。

## 積んだ未決（A）—— 1 件。答えが要る

- **Q6（第 2 回。premise）確認バッチの前の回の DB に、本体で消した記録が残る**（`review/code.md` R22。security review）。
  Q5 の答えで確認バッチは回ごとに別の DB に立ち、前の DB は残る。確認の間に電話が送った本物の記録はその回の DB にだけ入るので、
  残せば本体で消した記録が前の回の DB に残り、消せばその回の DB にしか無い記録を失う。
  推奨: 前の回の DB は残し、確認手順書に消し方の 1 行を載せ、design の Risks に書く（片付けの道具は後の change）。
  問いは `deep.md` Q6・`deep-questions-r2.json`・`docs/briefs/ST48-deep-r2.html`

## 仮決め（B）と Ruling

design の（仮）: **D5**（版の記録が読めないなら止める。付与の抜けは「読む権限が無い」として別の理由で返す —— R30）/ **D7**（指紋の違う版は当て直さず警告だけ）/
**D8**（組み立ての印は `CARGO_PKG_VERSION` と commit。試験は渡した値を `ASHIATO_EXPECT_BUILD_COMMIT` で受けて比べる —— R20）/
**D9**（配った印はタグ `v[0-9]*`。CI の取り寄せは `ls-remote` の失敗で落とす —— R18）/ **D10**（`202610250000` 以後の版の規則。`check-immutable.sh` の DB にも記録の置き場を作る —— R23）/
**D13**（確認バッチは回ごとの compose の project。`c01-clock` の反転条件は前の回の DB を数えて機械が判定 —— R27）/ **D17**（起動の検査はアプリの役割で、待ち受けの前に）。

全 Task の ledger の `Ruling:`（1 件。Task 8）:

- Ruling: 飛ばした DB を数えずに判定を出すまま park する — finish の fix は 1 回だけ（SDD）。数えた DB の名前と個数は note に出るので、本人の端末の回の DB が並ばなければ数え漏れと読める — 間違っていたときの費用: 反転条件（測り直しを 5→15 分）に当たっているのに「当たっていない」と出て、測り直しの見直しが 1 回の確認バッチの分だけ遅れる（記録は失われない）。直すなら飛ばした project を集めて note に載せ、1 つでもあれば判定を出さない

## Task

9 Task・16 項目を全部埋めた（`[x]` 16 / `[ ]` 0）。残した Task は無い。

## 検証（`evidence.jsonl`。finish の段で走らせ直したもの。どれも exit_code 0・PASS）

| 項目 | コマンド | 終了コード |
|---|---|---|
| 1.2 | `CT`/`NL` `st48_ledger_`・`st48_compare_`・`st48_read_ledger_unreadable_`（4 本に増えた） | 0 |
| 2.2 | `CT`/`NL` `st48_apply_` | 0 |
| 3.1 | `testdb.rs` の grep / `CT stay_tests` / `CT ingest` | 0 |
| 4.2 | `IT`/`NI` `st48_migrate_` | 0 |
| 5.2 | `IT`/`NI` `st48_server_`（拒否の 4 本は port を塞いで起動） | 0 |
| 6.1 | `check-released-migrations.sh --self-test` / 本体 | 0 |
| 8.1 | `verify-prep.sh` などの grep の組 | 0 |
| 8.2 | `tools/check-db-version-stop.sh`（89 秒）/ CI と testing.md の grep | 0 |
| 9.1 | `check_scenarios.py`（41 本）/ `cargo clippy --workspace --all-targets -- -D warnings` / `tools/smoke.sh`（136 秒）/ `tools/check-immutable.sh`（164 秒） | 0 |

ほかに `cargo fmt --all --check`・`cargo test -p ashiato-server --test server_startup`（25 本）・`tools/check-shared-appends.sh` も rc=0。
直した 6 件はガードをわざと壊して落ちることを確かめた（R19: 検査を `bind` の後へ → 拒否の 4 本が落ちる / R20: `build.rs` が渡した値を無視 → 落ちる / R30: 権限の検査を外す → 落ちる / R18: `origin` を壊す → rc=1）。

## レビュー（`review/code.md`。R1〜R32）

- final review（R1〜R17）: fixed 10・rejected 7（前の段で処置済み）
- code-verify（R18〜R21）: fixed 3（R18・R19・R20）・deferred ST58 1（R21）
- security review（R22）: escalated 1（上の Q6）
- `/code-review`（R23〜R32）: fixed 3（R23・R27・R30）・rejected 7（R24 は D7、R25 は D8、R26 は D10、R28 は R14 と同じ、R29 は spec の Scenario、R31 は CH04、R32 は D14 で決めたもの）
- finish の scoped re-review は 1 回で、R27 だけ PARTIAL（上の Ruling で park）

## 申し送り

- `docs/handoff/ST58.md`（新規）: 終了コード 3 は移行の失敗でない・その回の退避は無いので戻さない・migrate の前に古い S-01 を止める（R21）
- `docs/handoff/ST52.md`: 配った印のタグの型と組み立ての印（上流で書いた）
- 未処置: `tools/dev.sh` には `stack.sh` の「同じ port を持つ別 project の DB を止める」が無い（R17 で範囲外として rejected）。拠点内のバックアップと移行の前の退避に消した記録が残ることは要件の未決（Q6 の family。/requirements で拾う）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
