# ST57 — 新しい版が出たら知らせる

## Why

配布元（GitHub Releases）に新しい版を出しても、サーバ・携帯・PC の収集アプリは自分では気づかず、本人は古い版に取り残される（FR-112・FR-113）。
ただし更新の確認は外向きの通信になり、正典は「サーバに外部の宛先へ記録を送る部品を持たせない」と決めている（`openspec/specs/data-sensitivity/spec.md`・`tools/check-offsite.sh`。`docs/audit/2026-10-25-consistency.md` の C1）。
記録を 1 行も外へ出さずに、版の一覧だけを確かめる形が要る。ST58（「更新あり」を押して入れる）はこの Story の結果の上に乗る。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1 は失うものがある問い（`exported`。本人は答えを省けない）、ほかは仮でよい問い。
> 本人の答えが推奨と違えば `spec --answers` が書き直す。聞かない既定 C1〜C10（`defaults.md`）は design の D1〜D10 に写した。

- **3 つの部品がそれぞれ直に配布元に確かめる**（Q1 の推奨・FR-112）: サーバの更新係・携帯・PC の収集アプリが、最後に確かめてから 24 時間が過ぎた後の最初の機会に 1 回、`api.github.com` の `rai-256/ashiato` のリリースの一覧を取る。
  収集アプリはサーバに聞かない。問い合わせは `GET`・本文なし・固定の `User-Agent: ashiato-update-check` で、自分の版も端末の情報も送らない（FR-116）。宛先は組み立ての時の定数で、差し替えは試験の組み立てだけ
- **サーバの版は、サーバと別のプログラム・別のサービスの更新係が確かめる**（FR-112 ★）: 新しい実行ファイル `ashiato-updater.exe` をサーバの Windows の zip に入れ、インストーラがサービス `ashiato-updater`（`NT SERVICE\ashiato-updater`）として登録する。
  更新係は DB の部品を持たず、状態の置き場を読めず、結果を自分の置き場（`C:\ProgramData\ashiato-updater`）に書く。サーバはそこを読むだけ（書けない）。サーバの部品は変えない（`check-offsite.sh` はそのまま通る）
- **新しい版の決め方**（Q2 の推奨）: 入っている版のチャンネルで決める（正式版は正式版だけ、ベータ版はベータ版と正式版、開発版は印 `nightly` だけ）。下書きを捨て、自分の部品の配布物の名前から版を読む。3 つの部品で同じ材料（`docs/contracts/update-check-vectors.json`）で試す
- **画面**（Q7 の推奨・Q8 の推奨）: どの画面でも見出しの上の帯に 1 行で「更新あり 0.5.0（11/10 に見つけた）」を出し、設定の画面に「更新の確認」の節を置く。帯を押しても何も起きない（入れる口は ST58）。
  サーバに読みの口 `GET /update-status`（画面のログインが要る）を足す
- **切る口**: サーバはインストーラの `update-check off` / `on`（Q3 の推奨。管理者の確認 1 回・サービスを止めて無効にする）。携帯は設定の画面のスイッチ、PC は `update-check off` / `on` と `setup` の問い（Q4 の推奨。端末ごと。設定の文字列には入れない）。
  最初の問い合わせは切る口に触れられるようになった後（サーバはインストールの最後の表示の後、収集アプリは送り先を設定した後。C5）
- **知らせ方**: 携帯は最初の画面に出し、新しい版ごとに 1 回だけ通知する（Q5 の推奨）。PC は新しい版ごとに 1 回だけ通知する（Q6 の推奨）。
  7 日確かめられなければ「更新を確かめられていない」を同じ所に出す（Q9 の推奨）。携帯は網を問わずに確かめる（Q10 の推奨）
- **確認の無い前の版から入れ替えた収集アプリ**は、切り方を 1 回示すまで確かめない（深掘り Q11。携帯は通知を出せた時か画面に出た時、PC はトーストを出せた時に示したとみなし、どれでも示せない間は止まったまま。design D23・D24・D25（印を立てる時は Q12・Q13 で問い中）・D26（仮）。review/spec.md R16）
- **確かめた記録を追記で残す**（C4）。携帯の確かめた記録はバックアップと端末の移し替えから外す
- **インストーラ**: 走らせ直しで更新係を足し（切った確認は戻さない）、消す口で更新係のサービスも消す（置き場は残す）。`repair` は更新係に触れない（C7）

**要件の改訂（推奨のままなら record の段で行う）**: Q5（携帯の通知）と Q9（確かめられていない知らせ）の推奨は FR-113 に無いものを足す。本人が推奨のままにすれば、record の段が FR-113 を ★ で改訂する。この draft では `docs/requirements.md` と `docs/stories/ST57.md` に触らない。

**BREAKING ではない**: 送信の封筒・送信契約の版・取り込みの口・DB は変えない。**移行は無い**（DB に何も足さない。結果は更新係の置き場のファイル）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `distribution`: 更新の確認（3 つの部品の問い合わせ・問い合わせの形・版の決め方・時期と時計・更新係の隔離・結果と記録・切る口・画面の帯と設定の節・携帯と PC の知らせ・確かめられていない知らせ・網）を ADDED。
  `repair` の口が更新係に触れないことを ADDED。インストール・走らせ直し・消す口・Windows の配布物の中身・依存の許諾の一覧の 5 つの Requirement を MODIFIED（3 つめのサービスと実行ファイル・最初の問い合わせの時期・切った確認を戻さない・消す口で更新係も消す・サーバの Windows 版の一覧に更新係の部品）

**capability の照合**: INDEX.md は ST57 を `distribution`（「配布と更新（…更新の確認と適用と戻し…）」）だけに割り当てている。更新の確認の振る舞いは収集アプリと画面にもまたがるが、どれも更新の確認の一部として `distribution` に置く（`device-collection`・`desktop-collection`・`browsing-views` には振る舞いを足さない）。前倒しは無い。
画面の外への要求 0 件（`data-sensitivity` の「画面は外部の資源を読まない」）とサーバの外へ送る部品の検査（同「記録の写しを拠点外に置かない」）は変えず、`distribution` の Scenario で帯を出したときも守られることを見る。

**requires**: ST52（配布物の名前・3 つのチャンネル・版の出所）・ST55（インストーラ・サービス・置き場の並び `layout.rs`）はどちらも archive 済みで正典にある。

## Impact

- 新しい crate: `crates/update-check`（純粋な選び方・時期の判定・結果の形。HTTP の部品を持たない。サーバ・更新係・PC の収集アプリが使う）・`crates/updater`（`ashiato-updater.exe`。HTTP と TLS の部品を持つ。Windows のサービスの口は `crates/winsvc` を使う）
- サーバ（`crates/server`）: `GET /update-status`（更新係の置き場を読むだけ）。`docs/openapi.json` を作り直す。サーバの依存に HTTP の部品を足さない
- インストーラ（`crates/installer`）: `layout.rs` に更新係の列・`update-check off|on` の口・サービスの登録と ACL・走らせ直し・消す口
- PC の収集アプリ（`crates/collector-windows`）: `ureq` に TLS の機能を足す（design D12（仮）。`tools/check-licenses.sh`・`tools/check-vulns.sh`）・更新の確認と札 `update`・`update-stale` の通知・`update-check` の口・`setup` の問い・試験の組み立ての印 `ashiato_test_build`
- 携帯の収集アプリ（`collector-android`）: 組み立ての定数 `UPDATE_CHECK_URL`・更新の確認（`LocationService` の定期の時）・最初の画面の行・チャンネル `update` の通知・設定の画面のスイッチ・バックアップの除外（`res/xml/health_*`・`tools/check-android-backup-rules.sh`）
- 画面（`web/src`）: `Root.tsx` に帯・`SettingsView.tsx` に「更新の確認」の節・`update.ts`。e2e は `web/e2e/st57-update-notice.spec.ts`
- 道具と CI: `tools/release/build.sh`・`package.sh`・`check-artifacts.sh`（更新係を zip に入れる・差し込みの名前が無い）・新しい `tools/check-updater-deps.sh`（更新係が DB の部品を持たない・サーバが更新係に依らない）・新しい `tools/release/update-check-install.sh`（Windows の CI の `install-windows` で更新係の Scenario を見る）
- 文書: `docs/install.md`（更新の確認と切り方）・`docs/collector-setup.md`（収集アプリの切り方）
- 申し送り: `docs/handoff/ST58.md`（結果の形・押す口・切った確認を更新で戻さない）・`docs/handoff/ST60.md`（Linux の更新係の起こし方。C10）
- **走っている change との重なり**: 無い（`openspec/changes/` で archive でないのは st57 だけ）。後続の ST58 は requires に ST57 を持ち、`update-status` の結果と更新係の置き場の上に「押して入れる」を積む
