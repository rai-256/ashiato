# 深掘り（第 1 回）

> 推奨の答えで先に仕様と GO 資料を書いた（2026-10-20 の流れ）。答えは `deep-answers-1.txt`（2026-10-23）。Q1〜Q8 は本人がはっきり選び、すべて推奨と同じ（補足なし）。仕様への合意（QA）は「合意する」（`agreement.json`）。

### Q1. 再発止めの検査をどこに当てるか

- 問い: 手書きの一時フォルダを落とす検査（`tools/check-tmp-hygiene.sh`、`--self-test` 付き、CI の既存の検査の並びに足す）を、どのコードに当てますか。（出所: review/deep.md R12）
- 答え: Rust は crates/ の全体（製品のコードも）、Kotlin は app/src/test だけ。逃げ道は行の印 `tmp-ok: <理由>`（本人の選択。推奨と同じ）

### Q2. 検査が拾う書き方

- 問い: 検査は、どの書き方を落としますか。
- 答え: 作る書き方の兄弟の全部と、後始末を外す口も落とす（本人の選択。推奨と同じ）

### Q3. Rust と Kotlin 以外（bash の道具・/tmp の固定名・web の試験）

- 問い: tools/ の bash と、/tmp の固定名のファイルも、この change で直し・検査しますか。
- 答え: この change は Rust と Kotlin の試験だけ。bash も固定名も直さない（本人の選択。推奨と同じ）

### Q4. 走っている st09・st13 への響き方

- 問い: 検査が main に入ると、走っている st09 と st13 は rebase した後に、自分の足した手書きの一時フォルダで検査に落ちます。どう扱いますか。（出所: review/spec.md R5）
- 答え: 木の全体に当てる。st09・st13 は rebase の後、それぞれの gate → fix で直す（本人の選択。推奨と同じ）

### Q5. 「一時フォルダに残るものが増えない」の確かめ方

- 問い: 依頼 3 の「テストを一通り走らせた前後で /tmp に残るものが増えない」を、どう確かめますか。
- 答え: CI の既存の試験の段（ubuntu の `cargo test --workspace` と `:app:testDebugUnitTest`）で、一時フォルダを空のフォルダへ向けて走らせ、終わった後に空であることと走った試験の数が 1 以上であることを見る。手元の台本（`tools/tmp-leak-check.sh`）も同じことをし、下流の Task の検証にする（本人の選択。推奨と同じ）

### Q6. 今 /tmp にある残り

- 問い: 今 /tmp にある ashiato の試験の残り（`ashiato-export*` 196・`outbox*` 5,145・`st04*` 2,855・`st09-media*` 2,221 など、約 0.45G）を、この change で消しますか。（出所: review/deep.md R4）
- 答え: この change では消さない（仕組みにしない）。要るなら人が手で消すか、再起動（本人の選択。推奨と同じ）

### Q7. 殺された試験の残り

- 問い: 試験が殺されたとき（timeout・Ctrl-C の強制終了・OOM・Gradle の worker の kill）は、TempDir の drop も TemporaryFolder の後始末も走らず残ります。これをどう扱いますか。（出所: review/deep.md R5）
- 答え: 残ってよい（再起動か手で消す）。この change は普通に終わる・落ちる試験の後始末だけ（本人の選択。推奨と同じ）

### Q8. 失敗した試験の中身を後から見る口

- 問い: いまは失敗した試験の一時フォルダが残り、後から中身を覗けます。CH03 の後は消えます。残す口を持ちますか。（出所: review/deep.md R10）
- 答え: 持たない。中身が要るときはもう一度走らせて再現する（本人の選択。推奨と同じ）

## 問うた項目（第 2 回。finish から。2026-10-27）

| Q | 段 | 論点 | 失われるもの |
|---|---|---|---|
| Q9 | A（premise） | 8.3 の検証コマンドの入口を証跡の機械が拾えない（`review/code.md` R17） | なし（gate が 8.3 で止まる。直し方が harness2 か凍結した tasks.md か例外か） |

### Q9. 8.3 の検証コマンドの入口を、証跡の機械が拾えない（A / premise。推奨: harness2 の evidence.py に `openspec/` を足す）

- **なぜ人間の判断か**: plan_fix.py は旧コマンドの FAIL の証跡を要るが、旧コマンドが evidence.py:67 の CMD_LIKE に当たらず verify-run が走らないので、FAIL の証跡を作れない。機械の承認の道が成立しない
- **出所**: `review/code.md` R17（final の scoped re-review が R12 の残りを分けたもの。kind: premise）
- **問い**: `deep-questions-r2.json`（HTML は `docs/briefs/CH03-deep-r2.html`）
- 答え: (未回答)
