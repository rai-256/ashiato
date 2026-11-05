# ST54 — 開発版を正式版と並べて入れられる

## deep.md に積んだ未決（A）

- なし（`deep.md` に `(未回答)` は無い。finish で escalated にした指摘も無い）

## 仮決め（B）と Ruling

- **Task の ledger の `Ruling:`: なし**（st54-task-1〜11 の `progress.md` に `Ruling:` 行も `parked` も 0 件）
- design の仮決め（台帳 `provisional.jsonl`。別の席の確かめは merge gate の前にグラフが行う）:
  - D12（仮）系統の違いの知らせは ST50 と同じ口（携帯は画面の行と通知、PC は通知）で別の札・id。入った契機で 1 回、続く間は 24 時間ごとに出し直し、携帯の上限の 7 日前の知らせにも添える
  - D14（仮）系統は契約の版を読めた答えからだけ読み（404・読めない 200・401 は今どおり）、系統は開発版とそれ以外（正式版・ベータ版）の 2 つ。
    finish の R33（知らない見出しの値を正式版とみなす）はこの決定どおりとして rejected
- 本人が決めたこと（深掘り）: 第 1 回 Q1〜Q10（推奨のまま）・第 2 回 Q11〜Q14・第 3 回 Q15。D5・D7・D8・D13 は別の席が否とし、本人が決めた

## Task

- 全 11 Task を埋めた（実装の 20 項目は `[x]`）
- 残した項目（正常な残り）:
  - 8.1（`ci`）: Windows の実行時試験 `st54_side_by_side_autostart_and_aumid` は `tests/runtime_windows.rs` に置いた。Linux では組まれず、CI の Windows の job で走る
  - 12.1・12.2（`human`）: 開発版の下書きのリリース（nightly.yml を手で起こす）と、`docs/release.md` §9 の並べて入れる手順

## finish で処置した指摘

### final review（R1〜R21）

- fixed（c70b4d3e・8490cfad）: R1 実バイナリ試験が共有 DB を開発版に変えていた / R2・R8・R19 LINEAGE の読み方を 2 つの検査で揃えた / R3 カスタム版のサーバを版ごとに 1 回だけ組む / R4 開発版の migrate の全行の `-dev.` と続く起動の `/healthz` / R5 退避のログの置き場 / R6 知らせを消し損ねたら次の契機で消し直す / R7 系統の違いだけの行に日数を付けない / R9 doc コメント / R10 系統を 1 回求めて持つ / R12 PC のログを状態の変わり目の 1 回に / R13 重なる試験の整理 / R15 携帯の自分の系統の読み口を 1 つに / R17 公開手順が復元した鍵を使う / R18 一時の置き場を TMPDIR に / R20 試験の置き場 / R21 CI の fmt・clippy
- rejected: R11・R14・R16（設計 D14（仮）・D12（仮）と spec の「404 は系統の判定に使わない」どおり）

### code-verify・security review・/code-review（R22〜R38）—— 589ee7fb

- 試験が主張の階層を見ていなかったもの（壊しても全部緑だった）を直し、**同じ変異で落ちることを確かめた**:
  - R22 開発版のクッキー「だけ」: 同じ印の値を `ashiato_session` で付けて 401・Set-Cookie に `ashiato_session=` が無い（照合に正式版の名前を足す変異 → 1 failed）
  - R23 行の無い DB への**開発版の** migrate（unit と実バイナリ。行 0 を正式版とみなす変異 → 両方 1 failed）
  - R24 D3 の「最初の行」: 最初の行だけ系統の違う DB で、`applied_at` の順と名前の順の両方を見る（`DESC` の変異 2 通り → 1 failed）
  - R25 携帯の Drainer → 通知の配線: 試験の Drainer に本物と同じ `onChannelMismatch` を渡し、試験は通知の係を自分で呼ばない（配線を消す変異 → 2 failed）
  - R26 開発版の bin の migrate が渡した `MIGRATE_BACKUP_DIR` に退避を書き、渡した `BIND` で待ち受ける（置き場を無視する変異 → 1 failed）
- 直したもの:
  - R27・R30 `build.rs` が LINEAGE の行末の `# …` を外してから欄を割る（2 つのシェルの検査と同じ読み方。行末コメントつきの行で `cargo build` rc=0 を確かめた）
  - R28 `check-guards.sh` の Environment の名前を行全体で照合し、self-test に名前を変えた偽を足した（部分一致に戻すと self-test rc=1）
  - R29 ログの検査の許可語から使っていない `server` / `self` を外した（送り先の URL・端末識別子の抜け道だった）
  - R31・R36 携帯は系統の比べを `readContractAnswer` の 1 か所にし（`ContractAnswer.Version` から系統を外し、Sender・OriginalUploader・ContractGate の重複した枝を消した）、`channel_mismatch` のログを状態に入ったときの 1 回に間引いた（PC の R12 と同じ。間引かない変異 → 1 failed）
  - R32 更新の待ちと系統の違いが重なったときも、画面の行に「送り先が…のサーバです」を載せる（`screenNotice`。載せない変異 → 1 failed）
  - R34 PC の系統の知らせが、失敗の後に一度通れば次の失敗をまたログに出す（戻さない変異 → 1 failed）
  - R37 `docs/release.md` §9 に「正式版の `.env` を読み込んだ環境では `BIND=127.0.0.1:18788` を渡すか外す」を足した（明示の BIND が既定より先に立つのは spec どおり）
  - R38 開発版のリリース本文のリンクを絶対 URL にした
- rejected:
  - R33 知らない `Ashiato-Channel` の値を正式版とみなすのは D14（仮）の「系統は 2 つ」どおり
  - R35 開発版の PC 版の旧い置き場からの引き継ぎ: 開発版はまだ一度も配られていない（`git ls-remote --tags origin 'nightly*'` 0 行・リリース 0 件）
- scoped re-review 1 回: Critical・Important なし。R22〜R38 の `fixed` はどれも筋書きを塞いでいて、R33・R35 の反証も成り立つ。残した Minor 2 件（SDD の fix は 1 回だけなので直さずに残す）:
  - `st54_empty_ledger_allows_both_channels` の 2 周目の `UPDATE` は、直後の `DELETE` で消えるので効いていない。行が 0 なら系統は行の中身に依らないため、検査の強さは変わらない
  - R27 の提案の後半（build.rs 側に行末コメントの試験を置く）は入れていない。build.rs を行頭の `#` だけ外す形に戻しても、どの試験も落ちない（直しは手で 1 回確かめただけ）

## 通した検証コマンド

`evidence.jsonl` の最後の記録（全部 `PASS`）:

| 項目 | 本数 | 結果 | HEAD |
|---|---|---|---|
| 1.1 | 1 | PASS | 8184a7d9 |
| 2.1・3.1・4.1・4.2・5.1・6.1・7.1・10.2・10.3・11.1・11.2 | 24 | PASS（終了コード 0） | 589ee7fb（finish の直しの後に `scripts/verify-run` で取り直した） |
| 3.2 | 1 | PASS | 133e8a74 |
| 5.2 | 2 | PASS | d30e2dee |
| 6.2 | 1 | PASS | 12a55817 |
| 9.1・9.3（エミュレータ） | 2 | PASS | c00cdd25 |
| 9.2 | 2 | PASS | 589ee7fb |
| 10.1・10.4 | 2 | PASS | a6d63b8a |

9.2 には 8690a274 の FAIL が 1 行残っている。これは機械が承認して差し替える前の旧コマンド（`--tests *ContractGate*`）の記録で、`plan-corrections.md` で `*St50*` に直した。code-verify の手 7 で、新しいコマンドが 404 の扱いを壊すと落ちることを確かめた（`correction-review.md`）

ほかに手元で（589ee7fb の直しの後）: `cargo fmt --all --check` rc=0・`cargo clippy --workspace --all-targets -- -D warnings` rc=0・`cargo test -p ashiato-collector-windows` 362 passed・`cargo test -p ashiato-server --lib st5` 121 passed・`--test server_startup st54_bin_` 10 passed・gradle `:app:testDebugUnitTest :app:compileDebugAndroidTestKotlin` BUILD SUCCESSFUL・`check-log-private.sh` / `check-migrations.sh`（`--self-test` と本体）/ `check-nightly-migrations.sh --self-test` / `check-guards.sh --self-test` / `publish-nightly.sh --self-test` / `check_scenarios.py` / `review_triage.py` すべて rc=0

## 未処置の申し送り

- ST58: 終了コード 4（系統の食い違い）の回は退避から戻さない（`docs/handoff/ST58.md` の `st54-dev-side-by-side D3`。3.2）
- 印 `nightly` はまだ無いので、CI の `check-nightly-migrations.sh` は初回の公開まで「印なし」で通り、何も比べていない（spec どおり。開発版の公開を始めた後から効く）
- 開発版の鍵と `nightly` Environment の Secret・変数は、ST53 の確認の後に本人が置く（`docs/release.md` §9）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
