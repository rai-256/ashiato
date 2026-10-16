## ST08: PC のブラウザ履歴を集める

### deep.md に積んだ未決（A）

**なし。** 第 8 回 Q13（`review/code.md` R90。保存場所が残っていても前に送った版の記録がどちらも失われた後に時計が戻ったら、更新を取りこぼしてよいか）は
本人が推奨どおり「例外を広げる。読めないことは生存信号に出す」を選び（`deep-answers-8.txt`）、FR-13 の ★ 2026-10-14 訂正と、この finish の fix（R92）に入れた。

第 6 回 Q11（前に送った版より前にしない）・第 7 回 Q12（保存場所ごと消えた後は取りこぼしてよい）・第 8 回 Q13 はどれも回答済みで、実装と文書に入れてある。

### 仮決め（B）と Ruling

design の仮決め:
- **D3（仮）** 失敗した取得の試し直しは 1 分から倍々で 1 時間まで / 見回り 1 回で積む件数は 2,000 件 / 1 回の送信でウィンドウの記録を履歴より先に載せる（R69）
- **D10（仮）** 1 件の `vanished` は 1,000 件まで
- **D16（仮）** 送った版の最大値を帳面とは別の `browser-history/sent_version.json` に持ち、版をそれより前にしない（収集アプリで 1 つ）。
  「閉じないもの」（保存場所ごと消えた後と、それを広げた「前に送った版の記録がどちらも失われた後」）は第 7 回 Q12 / 第 8 回 Q13 で本人の決定になった。
  ファイルがあるのに読めない最大値は上書きせず `sent_version.broken.json` へ退避する。反転条件 (1)〜(3) は design D16
- **D17（仮）** 送った版の最大値が読めなかった生存信号（`history-sent-version-unreadable`）は「取得できない」（`capturable=false`）にする。
  読めた訪問は送れているので「取得できる」のまま種別だけを載せる案もある。**反転条件**: 記録が届いている日がこの 1 本で「取得できない」と読まれることが
  稼働状況・途絶の数え（FR-35 / NFR-13）で問題になったら、取得可否に効かない種別として載せる形に替える（整合の確認 2026-10-14 Q13 の C7）

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
- この finish（2 回目）の re-review Minor 5 件（R93〜R97）— Ruling: fix は 1 回の規則で直さず followup ST08 にした（Q13 の作りの堅さ: io エラーでも退避する・退避先は直近 1 つ・panic で種別が落ちる・テストのコメントの字面・退避できない経路のテスト）。誤りなら、一時的な読みのエラーで正しい床が退避され、帳面も失われていれば Q13 の例外の内で更新を取りこぼす
- R80 — 凍結された `tasks.md` の 10.1 / 10.2 の検証コマンドは本人の指示で訂正した（2a030d89、`tasks.md:181` の ★ 2026-10-14）。**merge のときに確かめてほしい点**

### Task の数

- **済: 11 / 11**（`[x]` 38 項目 / `[ ]` 0）。残した Task は無い
- 10.1 / 10.2（Windows の実行時テスト）は 2026-10-14 に Windows 実機で Edge・Chrome・Firefox の 3 本とも `1 passed`（code-verify 4 回目）。CI の `windows-latest` は gate が見る

### 通した検証

`evidence.jsonl` の各項目の最後の記録（Task gate が `scripts/verify-run` で走らせたもの）は、全項目 PASS（1.1〜11.6。10.1〜10.3 は 2026-10-14 の `1bd54906`）。

code-verify 4 回目（HEAD `e1ea86b2`）の実測: `cargo test --workspace` rc=0（collector 261・server 801 + 7）、clippy（Linux / Windows ターゲット）・fmt・`openspec validate --strict`・`check_scenarios.py`・`check_chain.py`・`review_triage.py`・`tools/smoke.sh`・`check-immutable.sh`・`check-migrations.sh`・`check-licenses.sh` すべて rc=0。

この finish（Q13 の fix、a420ab63）の後の木で手元で走らせたもの:

| コマンド | rc |
|---|---|
| `cargo test -p ashiato-collector-windows --lib`（264 passed） | 0 |
| `cargo clippy -p ashiato-collector-windows --all-targets -- -D warnings` | 0 |
| `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | 0 |
| `cargo fmt --all --check` | 0 |
| `openspec validate st08-browser-history --strict` | 0 |
| `python3 scripts/check_scenarios.py . st08-browser-history` | 0 |
| `python3 scripts/check_chain.py .` | 0 |
| `python3 scripts/review_triage.py . st08-browser-history`（指摘 127 件 / 仮決め 6 件） | 0 |

足したテスト `history_heartbeat_reports_unreadable_sent_version` は、種別を立てない変異・退避しない変異のどちらでも FAILED になることを確かめた。
server 側は触っていない（`blockers` の文字列は検査しない）ので、`cargo test --workspace` と `tools/smoke.sh` は code-verify 4 回目（上）の後に走らせ直していない。

### この finish での処置（`review/code.md`）

- **R89 fixed D16 / R90 escalated → 第 8 回 Q13（回答済み）/ R91 fixed D16**（code-verify 4 回目の 3 件。前回の finish）
- **R92 fixed D17 仮**（Q13 の答えと整合の確認 `docs/audit/2026-10-14-st08-fr13-q13-consistency.md` の C1〜C7・K1）:
  - コード: `read_sent_version` がファイルのあるのに読めない最大値を `sent_version.broken.json` へ退避し（退避できなければ送る準備の失敗）、
    生存信号の `blockers` に `history-sent-version-unreadable` を載せる。ファイルが無いだけ（初回・保存場所ごと消えた後）は載せない
  - spec: 識別子の SHALL の例外の括弧を Q13 へ広げ、生存信号に SHALL と Scenario を 1 本ずつ / design: D12・D16・D17（仮）・Context・Open Questions /
    送信契約: 履歴の `blockers` の語彙と `source_updated_at` の例外 / proposal: 第 8 回の表と FR-13 / handoff: R87 の括弧
  - D16 の「書けない」テストの仕掛けは、本体と同名のディレクトリ（いまは読めないとして退避される）から一時ファイルと同名のディレクトリに替えた
- **scoped re-review**（独立の reviewer、a420ab63）: C1〜C7・K1 はすべて入っている。Critical / Important 0。Minor 5 件（R93〜R97）は上の Ruling で park した

### 未処置の申し送り

- `docs/handoff/ST08.md`: R65 / R66 / R67 / R74 / R75 / R76 / R83 / R84 / R85 / R87 / R93〜R97（merge の後に `fix/` で拾う）
- R53 → `docs/handoff/ST28.md`、R86 → main 側の `fix/`（どちらも ST08 の差分ではない）
- `history_vanished_unknown_existence_is_present` は unix のみ（Windows では「在るか不明」の問い合わせを失敗させる手が無い）
