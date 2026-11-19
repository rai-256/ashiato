# CH23 — 毎晩の脆弱性の検査が issue を立てられず、9 晩赤のまま誰にも届かなかったのを直す

> 軽い路（`lane.json` の `reason: bug`。名指しの Scenario「落ちた夜に、依存ごとに 1 件の issue が立つ」）。深掘りと合意は省いてある。
> 受け皿の依頼 #333 の「議論で決めたこと」（何を・なぜ・どこまで）は本人の決定として扱う。

## Why

`vulns-nightly.yml` は 2026-11-10 から毎晩赤で、11-10〜11-18 の 9 晩は issue を立てる段（`tools/check-vulns.sh file-issues`）も
`error: issue を記録できなかった（gh の応答: Expecting value …）` で落ちていた。ラベル `脆弱性` が無いと、本物の `gh label list --search 脆弱性 --json name`
は `[]` でなく空の文字を返す（`premises.md`。gh 2.104.0 で実測）。`tools/check-vulns-issues.py` はそれを JSON として読めず、ラベルも issue も作らなかった。
自己試験の偽の `gh` は `[]` を返すので、正典の Scenario「落ちた夜に、依存ごとに 1 件の issue が立つ」の自己試験は通っていた。
その間、例外の表に無い当たり（PostgreSQL 17.6 → 17.12・libsqlite3-sys・chrono・rustix）が誰にも届かなかった（11-19 にラベルを手で作って #334〜#337 が立った）。

赤が気づかれなかったのは、赤の色そのものが知らせにならないから。勧告が残る夜は `vulns`・`pg-minor` の job が毎晩赤になり（11-19 の run も赤）、
予定した workflow の失敗の知らせは cron を最後に変えた人（本人）に送られる見込みだが、届いたかは確かめていない（`premises.md`）。
本人に届く経路は issue なので、**issue を立てる段そのものが落ちた夜**に、別の落ち方をする経路で知らせる。

## What Changes

- `tools/check-vulns-issues.py`: `gh label list` が空の文字を返したら空の一覧（ラベルが無い）とみなす（D1。`issue list` は無くても `[]` を返すと実測したので変えない）
- `tools/check-vulns.sh` の自己試験の偽の `gh`: `label list` は、ラベルが無いとき本物と同じく空の文字を返す（`issue list` は本物と同じ `[]` のまま）
- 知らせの段が落ちた夜の知らせ（ADDED）: `tools/check-vulns.sh report-unfiled <run の URL>` が、題 `脆弱性: 毎晩の知らせを issue にできなかった` の issue を
  ラベル無しで 1 件立てる（開いていれば、その夜の行を追記する）。`vulns-nightly.yml` の `issues` の job に、`file-issues` の段が落ちたときだけ走る段を足す（D2）
- 新しく立てる issue はリポジトリの持ち主を担当者にする（本人はこのリポジトリを watch しておらず、Actions が立てた issue の知らせが届かない見込み。D5）
- 配線の検査（`tools/check-vulns-wiring.sh`）がその段を見る。`docs/testing.md` の毎晩の行に書き足す
- 正典の Requirement と Scenario は変えない（`## MODIFIED` を書かない）。検査できなかった夜（rc=2）は正典のとおり issue を立てない（D3）

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `dependency-vulnerability-scan`: Requirement を 3 つ足す（ADDED のみ）

## Impact

- 触れる予定のパス（`lane.json`）: `tools/check-vulns-issues.py`・`tools/check-vulns.sh`・`tools/check-vulns-wiring.sh`・`.github/workflows/vulns-nightly.yml`・`docs/testing.md`・`docs/changes/CH23.md`（ADR は書かない）
- 触れないもの: 製品のコード・移行・API・収集アプリ・`osv-scanner.toml`（見つかった当たりの処置は #334〜#337 から別に扱う）・`nightly.yml`・`ci.yml`・`release.yml`
- 走っている change との重なり: `ch19-emulator-only-class`（`tools/android-emulator.sh`・`emulator-startup`）と `ch22-soak-install-online-ci`（`distribution`。origin/main にある）は、この change のパスと capability に重ならない（2026-11-20 に `openspec/changes/` の中を `check-vulns`・`vulns-nightly` で探して 0 件）
- 残るもの: 検査できなかった夜（osv.dev や版の一覧に届かない）は、正典の「Actions の赤だけで知らせる」のまま本人に届かないことがある（D3）
