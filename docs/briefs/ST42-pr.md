# ST42 — 衛星向けの API の契約を、版を付けて読める形で公開する（上流: 深掘り第 1 回の推奨で書いた仕様）

change: `openspec/changes/st42-satellite-api-contract/`（FR-89・FR-95・FR-103。扉 #31）

## 深掘りで本人が決めたこと

第 1 回の 7 問はどれも「仮でよい」問いで、答えの前に**推奨の答えで**仕様を書いた。答えが推奨と違えば `spec --answers` が写し直す。

- Q1 版は口の道に入れる（`/satellite/v1/…`）（推奨）
- Q2 壊す変更は、自前の台本が決めた規則で見分ける（推奨）
- Q3 比べる相手は、PR の行き先にある版ごとの写し。main に入った日から版 1 が凍る（推奨）
- Q4 確かめる口 `GET /satellite/v1/me` は、サーバが受ける版の一覧を返す（推奨）
- Q6 前の版を並べる 6 か月は、新しい版を最初に配った日（印）から数える（推奨）
- Q7 同じ版の中で足したら、小さい番号を上げる（1.0 → 1.1）（推奨）
- Q8 無い版・外した版は、版の一覧つきで断る。外す前は `Sunset` の見出しで予告し、本人に知らせる画面ができるまで版は外せない（推奨）

聞かない既定（`review/deep.md`「聞かない既定（C）」）: 契約を別の生成で分ける（D1）・Apache-2.0 は契約の欄と隣の LICENSE で示し、SPDX の行は置かない（D2）・版は製品の版と別に数える（D7）・MCP と封筒は載せない（D14）。

## 仮決め（design.md の（仮）。GO 資料で見て合意する）

- D3（仮） ページは入口と版ごとの 2 枚を契約から機械で作る
- D5（仮） 衛星向けの道の断りの本文と種別（`unauthorized`・`not_satellite`・`unsupported_version`・`not_found`・`method_not_allowed`・`internal`。`error` は文字列）
- D6（仮） 確かめる口の本文の形
- D8（仮） 版の一覧 `docs/satellite-api/versions.json` の形
- D9（仮） 壊す変更の規則の表
- D10（仮） 検査の台本 `tools/check-satellite-contract.sh`（比べる相手・`--generated`・CI の判定 `changes.sh satellite`。`code` の意味は変えない）
- D11（仮） 版の受け口の 3 本と、資格情報 → 版の判定の順・405・`Sunset` を付ける層
- D12（仮） 前の版を外す規則（配った日は印を作った日と commit の日の遅い方・暦の月・`notice` は正典の Requirement の名前）
- D13（仮） 衛星向けの道の読み出しの記録（ハンドラの後に返す状態で書く・書けなければ 500）

## capability

- `satellite-platform`: ADDED 7 本・MODIFIED 1 本（「衛星の資格情報では、まだどの口も通らない」を、衛星向けの道だけを例外にする形に）
- `data-sensitivity`: MODIFIED 2 本（「API の合言葉で通る口は…」から衛星向けの契約の口を外す・「誰がいつ読んだかを…」の「（403）のとおり」に 404・405 を足す。後者は ST59 も書き直すが、直す文は別で archive の 3-way で合わせる）
- 前倒しは無い。INDEX の `data-sensitivity` の行に ST42 を足した。`record-envelope`（FR-103 の封筒の形）は ST39 が同じ版に追加として載せる（deferred ST39。`docs/handoff/ST39.md`）
- 走っている change との重なり: ST61（`changes.sh`・`ci.yml`・`check-openapi.sh`）・ST59（router・middleware・data-sensitivity）・CH21 / CH26（`ci.yml`）。どれも同じ行を取り合わない形にした（proposal の Impact）

## tasks

9 Task・12 項目（人間の確認待ちは無い。画面は持たない）。ST42 が印を置く Scenario は 59 本（Rust 21・台本の本番 4・台本の `--self-test` 34）。

## レビュー

- 深掘りの独立レビュー（`review/deep.md`）: 15 件（escalated 4・fixed 10・deferred 1）
- spec の独立レビュー（`review/spec.md`）: 20 件。fixed 19（うち conflict の R1・R18 は失う側が無いので `fixed D12 仮`・`fixed D10 仮`）・deferred ST39 1。R11 の題の付け替え（RENAMED）は、題が正典の別の本文や申し送りから名前で引かれているのでせず、本文を直した

🤖 Generated with [Claude Code](https://claude.com/claude-code)
