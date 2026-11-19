# CH23 — 毎晩の脆弱性の検査が issue を立てられず、9 晩赤のまま誰にも届かなかったのを直す（上流・軽い路）

軽い路（`reason: bug`。名指しの Scenario「落ちた夜に、依存ごとに 1 件の issue が立つ」）。深掘りと合意は省いてある。仕様・spec-review・Task ごとの実装とレビュー・merge gate・確認バッチは残る。

## 本人が決めたこと（受け皿の依頼 #333 の「議論で決めたこと」）

- `tools/check-vulns-issues.py` が、gh の空の応答を空の一覧とみなす。自己試験の偽の gh を、本物の返し方に合わせる
- 赤が 9 晩気づかれなかった理由（知らせがどこに届くか）を調べて、届く形にする
- 見つかった当たり（PostgreSQL 17.6 → 17.12・libsqlite3-sys・chrono・rustix）の処置は別に扱う（#334〜#337）。ラベルが無い状態でも動くことを試験で確かめる

## 調べたこと（`premises.md`）

- 本物の gh 2.104.0 は、当てはまるラベルが無いと `label list --json` に空の文字（0 バイト・rc=0）を返す。`issue list` のほうは `[]` を返す（どちらも実測）
- 予定した workflow の知らせは、cron を最後に変えた人（本人）に送られる（github/docs の原稿の逐語）。届いたか・埋もれたかは確かめていないので、確認バッチで聞く
- 勧告が残る夜は、検査の job が毎晩赤になる（11-19 は issue が立った後も赤だった）。だから赤の色は知らせにならない

## 仮決め（`design.md`）

- **D1（仮）**: 空の文字を空の一覧とみなすのは、実測した `label list` だけ
- **D2（仮）**: `file-issues` の段が落ちた夜は、同じ `issues` の job の `report-unfiled` で知らせる。題は `脆弱性: 毎晩の知らせを issue にできなかった`、ラベルは付けない。開いていれば追記し、落ちた run ごとに 1 回書く
- **D3（仮）**: 検査できなかった夜（rc=2）は、正典のとおり issue を立てない。変えると MODIFIED になり、軽い路の外。確認バッチで本人に聞く
- **D4（仮）**: job の赤と緑の付き方は変えない

## 規模

- specs: `dependency-vulnerability-scan` に Requirement を 2 つ足す（ADDED のみ。Scenario は 12）。capability の前倒しはしない
- tasks: 3 Task・7 項目と、人間の確認待ち 1 つ（確認バッチの問いが 3 つ）
- spec-review: 指摘 11 件。technical 9 件・defer 1 件・premise 1 件で、11 件とも fixed。premise の 1 件は、前提を公式文書の原稿で確かめてから technical に分け直した。A（人間に返す問い）は 0 件

## 残るもの

- 検査できなかった夜は、赤だけで本人に届かないことがある（D3）
- gh そのもの・権限・GitHub の障害で知らせの段も落ちた夜は、赤だけが残る（D2）
