# CH03 テストの一時ファイルの後始末（上流）

change: `openspec/changes/ch03-test-tmp-cleanup/`。新しい capability `test-hygiene`（ADDED 3・Scenario 18 本）。依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH03.md`。

> **第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた**（2026-10-20 の流れ）。下の Q1〜Q8 は**本人の答えではなく推奨**で、`deep.md` は `（答え待ち・推奨: …）` のまま。
> 答えが推奨と違えば `spec --answers` が仕様に写し直す。全問が B（仮でよい）。

## 深掘り（第 1 回。推奨で書いた）

| Q | 推奨（仕様はこれで書いた） | どこに効くか |
|---|---|---|
| Q1 検査をどこに当てるか | Rust は `crates/` の全体（製品のコードも）、Kotlin は `app/src/test` だけ。逃げ道は `tmp-ok: <理由>` | spec Requirement 2、design D3、tasks 7.1 |
| Q2 検査が拾う書き方 | 作る書き方の兄弟の全部と、後始末を外す口（`keep()`・`into_path()`・`persist(`・規則の無い `TemporaryFolder()`）も落とす | spec Requirement 2、design D4、tasks 7.1 |
| Q3 bash・`/tmp` の固定名 | 直さない（Rust と Kotlin の試験だけ） | design Non-Goals |
| Q4 走っている st09・st13 | 木の全体に当てる。rebase の後、それぞれの gate → fix で直す（spec-review R5 で「コンパイルも通らなくなる」を context に足した。推奨は変えない） | proposal Impact、design D1・D2・D5 |
| Q5 残りの確かめ方 | CI の既存の試験の段を、一時フォルダを空のフォルダへ向けて走らせ、空であることと試験が 1 本以上走ったことを見る | spec Requirement 3、design D6、tasks Task 1・8 |
| Q6 今 `/tmp` にある残り | 消さない | design Non-Goals |
| Q7 殺された試験の残り | 扱わない | spec Requirement 1 の本文、design Non-Goals |
| Q8 失敗した試験の中身を残す口 | 持たない | proposal（変わってしまう振る舞いの 1 つ） |

聞かずに決めたもの（`premises.md` の C1〜C4）: Rust は `tempfile::TempDir`（手伝いは `TempDir` を持つ型を返す。D1）/ Kotlin は `@get:Rule` の `TemporaryFolder`（手伝いはフォルダを引数で受け取る。D2）/
何を使うかを `docs/testing.md` に書く（D8）/ 試験の期待値・印・名前は変えない（D7）。

## 仮決め（B）と反転条件（design の（仮））

- **D3（仮）** 検査の範囲はファイルの置き場で決め、`#[cfg(test)]` を読まない。反転条件: Q1 の答えが「androidTest も」「試験のコードだけ」なら範囲を変える
- **D4（仮）** 落とす書き方の表と、同じ行か 1 行上の印。反転条件: Q2 が「依頼の 3 つだけ」なら絞る／`tmp-ok` が 1 Story に 3 つを超えて要るなら表を見直す
- **D5（仮）** 検査は CI の chain job と commit 前（追跡ファイル全体）。反転条件: rebase の後の直しを 2 回以上妨げたら commit 前は staged だけ／chain job で気づくのが遅れて 2 回直しが遅れたら rust job にも
- **D6（仮）** 残りは `target/tmp-leak/` の下の空のフォルダへ向けて測る（cargo は `--no-run` の後に `TMPDIR`、Gradle は `ASHIATO_TEST_TMPDIR` → `java.io.tmpdir` と `--rerun`）。反転条件: Robolectric・Gradle・cargo が自分のものを残すなら名前の頭の一覧で外す／CI が 10% を超えて伸びたら `--rerun` をやめる

仮でない決定: D1（`TempDir` を最後まで持つ。`Inbox` は形を残す）・D2（`TemporaryFolder` と `by lazy` / `@Before`）・D7（差分の印・名前・本数を 8.3 が機械で見る）・D8（docs）。

## capability

`test-hygiene` を新しく立てた（道具の capability は初めて）。schema の「tooling は `skip_specs`」を採らない理由は proposal の Capabilities（外から見える振る舞いを足し、正典で Scenario を印に縛るため）。
`docs/changes/CH03.md` の `capability: []` はそのまま（新しく、走っている Story と重ならない）。

## tasks

8 Task・25 項目。Scenario 18 本は全部どこかの Task が名指しする。画面は持たないので e2e と人間の確認待ちは無い。
`tools/check-tmp-hygiene.sh`・`tools/tmp-leak-check.sh`（どちらも `--self-test` 付き）を足し、CI の rust・android・chain の 3 job と `tools/pre-commit.sh` を変える。製品のコードは変えない。

## レビュー

- deep-review（`review/deep.md`）: 15 件 —— fixed 11・escalated 4（問いに足した・直した: Q1・Q6・Q7・Q8）
- spec-review（`review/spec.md`）: 15 件 —— fixed 12（うち仮 1: D5）・escalated 1（R5 → Q4 の context）・rejected 1（R11: capability を残す）・kind の直し 1（R15 daily → technical）
  - 主なもの: 略記が verify-run に読めず依頼 3 の検証が走らない形だった（R2）→ 略記を直し、`evidence.declared()` で全項目にコマンドがあることを確かめた。
    Kotlin の手伝いを property の初期化で呼ぶ 10 ファイル（R4）、走っている st09・st13 のコンパイルが通らなくなる（R5）、1.4 が後で走らせ直すと落ちる形だった（R6）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
