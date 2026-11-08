# CH11 — 依存の脆弱性を CI で検査する（上流）

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。本人の答えはまだ無い。推奨と違う答えが返ったら、`spec --answers` がその問いだけを写す。
> リポジトリは公開で、公開の場に何を書くか（Q10）は答え待ち。**この PR の文書には、直っていない依存の名前・版・勧告の ID を書いていない**（数と種類だけ）。

## 何をするか

依存の脆弱性を見る検査が 1 つも無い（許諾と秘密の検査はある）。osv-scanner 1 本で Rust・web・携帯（gradle）のロックファイルを見て、
PR（依存に関わるファイルが変わったときだけ・新しく持ち込んだ勧告で落とす）・毎晩（main の全体・落ちたら依存ごとに issue）・印 `v*`（全体の検査が通るまで組まない）で機械が止める。
逃がすのは理由と期限（90 日以内）つきの例外の表 `osv-scanner.toml` 1 つだけ。携帯に gradle の依存の固定を入れ、同梱の PostgreSQL の minor の遅れも毎晩見る。

## 深掘り（第 1 回。12 問・推奨で先に書いた）

A（止める問い。本人は答えを省けない）:
- Q8 配る経路を止めるか → 推奨: 印 `v*`（正式版とベータ版）の組み立ては止め、開発版の公開は止めない（D16（仮））
- Q10 公開の場に何を書くか → 推奨: issue と例外の理由に書くのは依存・版・勧告の ID・重大度と、「届かない理由」か「上げる予定」だけ（D5（仮））

B（推奨のままなら仮決め）:
- Q1 いまある勧告 → 互換の範囲で上げ、残りは例外の表へ（D18（仮））
- Q2 PostgreSQL 17.6 → この change では上げない。毎晩の検査が issue を立てる（D15（仮））
- Q3 PostgreSQL の「最新」→ postgresql.org の一覧。EDB に zip があるか、公開から 30 日で落とす（D14（仮））
- Q4 開発用の依存 → 区別せず落とす（D3（仮））
- Q5 重大度 → 問わず落とす。RUSTSEC の知らせも（D2（仮））
- Q6 例外の表 → 1 つ。dependency-review の許可は表から CI が組む（D10（仮））
- Q7 期限の上限 → 検査した日から 90 日以内（D4（仮））
- Q9 AI が例外の行を足してよいか → 足してよい。PR の検査と PR 本文に一覧で出す（確認バッチへの表示はハーネス側の変更が要り、この change の外。D9（仮））
- Q11 GitHub の設定 → 下流の AI が `gh` で有効にし、API で読み返す（D17（仮））
- Q12 携帯の固定 → 全部の構成とプラグインの classpath を STRICT で（D11（仮））

## 仮決め（design.md の（仮））

- D2〜D5・D9〜D11・D14〜D18（仮）: 上の Q の推奨。反転条件は design.md の各 D
- D20（仮）: PR の脆弱性の検査は、ロックファイル・例外の表・検査の台本が変わったときだけ。workflow だけの変更では走らせず、網に依らない配線の検査をコードの検査の job で走らせる（spec-review R11）

聞かずに決めた既定（`premises.md` の C1〜C10 → design の D1・D6〜D8・D12・D13・D19〜D21）。

## capability

- 新しい capability `dependency-vulnerability-scan`（開発と配布の道具の約束。CH09 の `test-timing`・CH10 の `emulator-startup` と同じ扱い）。前倒しは無い。MODIFIED は無い（正典の `distribution` は変えない）

## tasks

11 Task・22 項目。Scenario は 50 本（`tools/check-vulns.sh --self-test` 35・`tools/check-vulns-wiring.sh` 5・`tools/check-pg-minor.sh --self-test` 6・gradle の固定 3・GitHub の設定 1）。
自己試験は使い捨ての木と偽の `gh` で走り、本物の GitHub に触らない（osv.dev への問い合わせは要る）。
PR の CI の結果・merge の後の最初の毎晩・（下流が済ませられなければ）GitHub の設定は「人間の確認待ち」（確認バッチ）。

## レビュー

- 深掘りの問いの独立検査（`review/deep.md`）: 15 件。A の問いの追加 1（Q8）・Q10 の広げ直し・前提の直し。処置の字面を spec の段で直した（escalated 2・仮 1）
- spec-review（`review/spec.md`）: 18 件（technical 15・conflict 2・daily 1）。全部 fixed（仮 5: D5・D9・D14・D16・D20）。人間に返す新しい問いは無い。
  kind を 2 件直した（R2 irreversible → technical: 失うものが無い。R6 premise → conflict: 問いの字面と spec の食い違いで、問いの説明を直した）
- 走っている change との重なり: st56・ch10 は `ci.yml` の別の行。st53（実装は main）の `tools/check-licenses.sh` の自己試験は、写した木のロックの扱いだけ直す。`spec_overlap.py check` は 0 件
