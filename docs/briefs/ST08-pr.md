## ST08: PC のブラウザ履歴を集める

### deep.md に積んだ未決（A）

**1 件: 第 8 回 Q13（未回答）** —— `docs/briefs/ST08-deep-r8.html`（`openspec/changes/st08-browser-history/deep-questions-r8.json`）。

- **Q13**（irreversible / loss: uncaptured。`review/code.md` R90）: 保存場所は残っているのに、送った版の手がかり（`sent_version.json` と帳面）がすべて読めなくなった後に時計が戻っていたとき、更新を取りこぼしてよいか。
  第 7 回 Q12 の例外（保存場所ごと消えた後）の**外**で同じことが起きると、code-verify 4 回目の一時テストで実測した（戻った後の版が前に送った版より 400 日古くなり、壊れた床のファイルが低い値で上書きされる）。
  推奨は「例外を広げ、読めないことを生存信号に出す」。どちらの答えでも、読めないファイルの退避と生存信号の種別は答えの記録の後の fix で入れる

第 6 回 Q11（前に送った版より前にしない）と第 7 回 Q12（保存場所ごと消えた後は取りこぼしてよい）は回答済みで、実装と文書に入れてある。

### 仮決め（B）と Ruling

design の仮決め:
- **D3（仮）** 失敗した取得の試し直しは 1 分から倍々で 1 時間まで / 見回り 1 回で積む件数は 2,000 件 / 1 回の送信でウィンドウの記録を履歴より先に載せる（R69）
- **D10（仮）** 1 件の `vanished` は 1,000 件まで
- **D16（仮）** 送った版の最大値を帳面とは別の `browser-history/sent_version.json` に持ち、版をそれより前にしない（収集アプリで 1 つ）。
  「閉じないもの」（保存場所ごと消えた後）は第 7 回 Q12 で本人の決定になった。反転条件 (1)〜(3) は design D16

全 Task の ledger の `Ruling:`（`.superpowers/sdd/st08-task-*/progress.md`。Task 11 にだけある）:
- parked R65 — Ruling: 直さずに followup ST08 にした（Minor。失うものは無い）。誤りなら、まれな構成（ini の絶対パスの名前と Profiles\ 直下の名前が一致する）で誤った vanished が取得のたびに積まれ、2 つのプロファイルの訪問が同じ profile_dir に混ざる
- parked R66 — Ruling: Q8 の文面どおり URL 無しの識別子で送り直すことを許し、followup ST08 にした。誤りなら、URL の行を失った訪問が取り込み口で二重に数えられる
- parked R67 — Ruling: テストの理論上の不安定として followup ST08 にした。誤りなら、テストがまれに落ちるか、コードの誤りが行の不正として黙って飛ばされる
- Ruling: R61（push と CI）はこの段で扱わず、merge_gate の CI 待ちに任せた。誤りなら、10.4 の [x] が Windows の job で一度も緑を見ないまま残る（gate が赤なら draft に戻す）
- Ruling: R58 の loss: exported は人間への問いではなく不具合の影響として扱い、fixed で閉じた（D11 は決まっている）
- parked R74 — Ruling: 深掘り第 5 回 Q9 の実装の re-review の Minor として followup ST08 にした（fix は 1 回）。誤りなら、取り込み口が止まって積み込みが途中で落ち続ける間 outbox.jsonl が試すたびに伸び、書きかけの断片に次の追記が連結されて再起動時に退避に回る
- parked R75 — Ruling: 振る舞いの変化（消えたプロファイルの帳面を毎回書く・除外の登録を読み終えた直後に読む）を害なしとして followup ST08 にした。誤りなら、積み終えるまでに足した除外がその回の取得に効かず、除外したはずの訪問が 1 回送られる
- parked R76 — Ruling: テストの不足として followup ST08 にした。誤りなら、2 つめ以降のまとまりの後で帳面を書く誤りや帳面の保存の失敗の数え漏れが、テストを緑のまま入りうる
- この finish の re-review Minor 4 — Ruling: Q13 の問いの「送った版を覚えているファイル 2 つ（送った版の最大値と帳面）」の「帳面」がプロファイルごとに複数ありうることを、文言で書き分けずに park した —— 「すべて読めなくなった」で範囲は足りている —— 誤りなら、答える人が帳面は 1 つだと思って起きる条件を実際より狭く読む
- R80 — 凍結された `tasks.md` の 10.1 / 10.2 の検証コマンドは本人の指示で訂正した（2a030d89、`tasks.md:181` の ★ 2026-10-14）。**merge のときに確かめてほしい点**

### Task の数

- **済: 11 / 11**（`[x]` 38 項目 / `[ ]` 0）。残した Task は無い
- 10.1 / 10.2（Windows の実行時テスト）は 2026-10-14 に Windows 実機で Edge・Chrome・Firefox の 3 本とも `1 passed`（code-verify 4 回目）。CI の `windows-latest` は gate が見る

### 通した検証

`evidence.jsonl` の各項目の最後の記録（Task gate が `scripts/verify-run` で走らせたもの）は、全項目 PASS（1.1〜11.6。10.1〜10.3 は 2026-10-14 の `1bd54906`）。

code-verify 4 回目（HEAD `e1ea86b2`）の実測: `cargo test --workspace` rc=0（collector 261・server 801 + 7）、clippy（Linux / Windows ターゲット）・fmt・`openspec validate --strict`・`check_scenarios.py`・`check_chain.py`・`review_triage.py`・`tools/smoke.sh`・`check-immutable.sh`・`check-migrations.sh`・`check-licenses.sh` すべて rc=0。

この finish の後の木で手元で走らせたもの:

| コマンド | rc |
|---|---|
| `cargo test -p ashiato-collector-windows --lib`（263 passed） | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | 0 |
| `cargo fmt --all --check` | 0 |
| `openspec validate st08-browser-history --strict` | 0 |
| `python3 scripts/check_scenarios.py . st08-browser-history` | 0 |
| `python3 scripts/check_chain.py .` | 0 |
| `python3 scripts/review_triage.py . st08-browser-history`（指摘 121 件 / 仮決め 5 件） | 0 |

足したテスト 2 本は、ガードを外す変異で落ちることを確かめた（V4: 送った版を書けない失敗を `.ok()` で無視 → unwritable のテストが FAILED / V3: 帳面の各訪問の前の版を床に使わない → without_sent_version のテストが FAILED）。

### この finish での処置（`review/code.md`。code-verify 4 回目の 3 件）

- **R89 fixed D16**: 第 7 回 Q12 の例外を spec の識別子の SHALL の括弧と導出元・理由、design（Context・D16 の「閉じないもの」を本人の決定に・Open Questions）、`docs/collector-contract.md` の `source_updated_at`、proposal（第 7 回の表・FR-13 ★ 2026-10-14）、`docs/handoff/ST08.md` の R87 に入れた（整合の確認 2026-10-14 の C1〜C5）
- **R90 escalated** → 第 8 回 Q13（上）
- **R91 fixed D16**: `history_heartbeat_counts_unwritable_sent_version_as_not_queued` / `history_runtime_clock_back_update_without_sent_version_has_newer_version`
- **scoped re-review**（独立の reviewer）: R89・R90・R91 とも ADDRESSED。Important 0。Minor 4 件のうち 3 件（Q13 の文言: 実測と推論の区別・選択肢 2 が Q12 の例外も閉じること・退避と種別はどちらの答えでも入れること）は直し、1 件は上の Ruling で park した

### 未処置の申し送り

- `docs/handoff/ST08.md`: R65 / R66 / R67 / R74 / R75 / R76 / R83 / R84 / R85 / R87（merge の後に `fix/` で拾う）
- R53 → `docs/handoff/ST28.md`、R86 → main 側の `fix/`（どちらも ST08 の差分ではない）
- `history_vanished_unknown_existence_is_present` は unix のみ（Windows では「在るか不明」の問い合わせを失敗させる手が無い）
