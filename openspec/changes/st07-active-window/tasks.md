# ST07 実装タスク — PC のアクティブウィンドウを集める

読む順: `deep.md`（**最優先。本人が決めた 8 件**）→ このファイル → `specs/desktop-collection/spec.md`
→ `design.md` → `docs/stories/ST07.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行の名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`**（連番にしない。作った時刻を付け、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す）。
**`collection-coverage` と `record-envelope` には触らない**（ST02 / ST03 の capability）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` と `tools/seed.sh` が前提。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: アプリを切り替えると 1 件増える`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、
  **印の無い Scenario を FAIL にする**。`scripts/merge_gate.sh` がこの検査を見るので、
  **印を置かないと tasks を全部チェックしても PR は止まる**
- 印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **この change が足す Scenario は 23 本**（すべて `desktop-collection`）
- 実機（Windows の実環境）でしか確かめられないものは、下の「人間の確認待ち」節に挙げる。
  **挙げたものだけが検査を通る**
- **D5 / D7 / D8 / D9 は（仮）決め。** 反転条件は `design.md` にある。
  実測で値を変えたら、その D 番号の（仮）を外すか反転条件を書き直す

## 1. 登録簿と移行

- [ ] 1.1 `migrations/<作成時刻>_c02_window_source.sql` を作り、`core.source` の `c02-window` を
  `external_id_kind='none'` にする（design D2）。**`external_id_kind` 列がまだ無い場合は何もしない
  分岐**を置く（ST03 の移行の当たる順に依存しないため）。検証: `psql -c "SELECT external_id_kind
  FROM core.source WHERE logical_source='c02-window'"` が `none` を返すか、列が無ければ
  移行が rc=0 で通る。`cargo test --test migrations` rc=0
- [ ] 1.2 戻し手順 `.down.sql` を書く。**`'record'` に戻すとウィンドウの記録が全件 400 になる**ので、
  列があるときだけ・`RAISE WARNING` つきにする。検証: `psql < migrations/<作成時刻>_c02_window_source.down.sql`
  が rc=0 で、警告が出力に含まれる
- [ ] 1.3 `tools/check-migrations.sh` を通す。検証: `tools/check-migrations.sh` rc=0
- [ ] 1.4 `crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。検証:
  `cargo test --test migrations` rc=0（当て直しても表が壊れない）

## 2. 送る形（契約）

- [ ] 2.1 `docs/collector-contract.md` に **C-02 が送る `payload` の形**を追記する（design D1）——
  アプリ名（表示名）・実行ファイルのパス・プロセス名・ウィンドウ題名・URL・記録の種類
  （`foreground` / `idle` / `powered-off` / `excluded`）・範囲の終わり（`idle` と `powered-off` のみ）。
  **`raw` は収集側が組んだ JSON を文字列のまま送る**ことを明記する。
  検証: `grep -c "c02-window" docs/collector-contract.md` が 1 以上、
  `python3 scripts/check_chain.py .` rc=0
- [ ] 2.2 直列化の形を Rust 側のテストで固定する（**形が変わると同じ 1 件が別の鍵になる**。design D1）。
  検証: `cargo test payload_shape_is_pinned` rc=0

## 3. 前景の変化を拾う

- [ ] 3.1 `crates/collector-windows` に前景の読み取りを実装する。OS のイベント通知と
  1 秒間隔の見回りを併用する（design D5・**仮**）。検証:
  `cargo test --package ashiato-collector-windows foreground` rc=0
- [ ] 3.2 アプリ名・ウィンドウ題名・URL のいずれかが変化したら記録を 1 件作る。
  Scenario: `アプリを切り替えると 1 件増える` / `同じアプリの中で題名が変われば 1 件増える` /
  `URL だけが変われば 1 件増える`。検証: `cargo test --package ashiato-collector-windows` rc=0
- [ ] 3.3 URL は**前景がブラウザのときだけ** UI Automation でアドレスバーを読む（design D4 / Risks）。
  **補正しない**（design D12）。Scenario: `クエリとフラグメントが残る` / `表示されている文字列を補正しない`。
  検証: `cargo test url_is_not_normalized` rc=0
- [ ] 3.4 題名だけの変化に最小滞留 5 秒を置く。**アプリと URL の変化は滞留に関わらず必ず 1 件**
  （design D8・**仮**）。Scenario: `題名が最小滞留より短く変わり続けても記録は増えない` /
  `アプリの切り替えは滞留時間に関わらず記録される` / `URL の変化は滞留時間に関わらず記録される`。
  検証: `cargo test min_dwell` rc=0

## 4. 離席（FR-81）

- [ ] 4.1 最後の入力からの経過時間・画面ロック・スリープを読む。閾値は 5 分（design D9・**仮**）。
  **経過時間も記録に載せる**（閾値を変えたときに引き直せるようにするため）。
  Scenario: `離席の始まりと終わりが残る` / `画面ロックとスリープも残る`。
  検証: `cargo test idle_transitions` rc=0
- [ ] 4.2 離席の記録を前景の記録と区別できる形にする（`payload` の記録の種類。design D1）。
  Scenario: `離席の記録は前景の記録と区別できる`。検証: `cargo test record_kind_is_distinguishable` rc=0

## 5. PC が止まっていた期間（FR-82）

- [ ] 5.1 停止時刻の印をローカルに書き、**動作中は 1 分ごとに更新する**（design D10 / Risks）。
  検証: `cargo test last_seen_marker` rc=0
- [ ] 5.2 起動時に、印と現在時刻から区間の記録を 1 件作る。**印が無ければ書かない**。
  Scenario: `起動時に止まっていた期間が 1 件残る` / `初回起動では生成しない`。
  検証: `cargo test powered_off_span` rc=0
- [ ] 5.3 この記録が**意図的な停止（FR-34）と区別できる**ことを固定する（design D10）。
  Scenario: `意図的な停止と区別できる`。検証: `cargo test powered_off_is_not_stopped` rc=0

## 6. 除外（FR-83）

- [ ] 6.1 除外する対象の登録の形（実行ファイルのパス / プロセス名 / ウィンドウ題名の一致規則）を作る。
  **既定は空**。検証: `cargo test exclusion_empty_by_default` rc=0
- [ ] 6.2 除外の判定を**送る前**に行い、本文を持たない記録を 1 件書く（design D11）。
  Scenario: `除外に登録した対象の本文は残らない` / `除外した件数が残る` / `除外の登録が空なら何も除外されない`。
  検証: `cargo test exclusion` rc=0
- [ ] 6.3 **除外リストの初期登録の手順**を `crates/collector-windows/README.md` に書く
  （design Risks の「URL にトークンが入る」の守りがこれと PERM-9 の 2 つしかないため）。
  検証: `grep -c "除外" crates/collector-windows/README.md` が 1 以上

## 7. 生存信号（FR-78）

- [ ] 7.1 想定間隔（登録簿の `c02-window` = 21600 秒）ごとに生存信号を送る。
  Scenario: `想定間隔ごとに生存信号が届く`。検証: `tools/smoke.sh` に手順を足して rc=0
- [ ] 7.2 取得可否を**前景ウィンドウが取れること**と **UI Automation が応答すること**で判定し、
  満たされていない側を `blockers` に載せる（design D4）。
  Scenario: `前景を読めない状態は取得できないとして報告される`。検証: `cargo test capturable_blockers` rc=0
- [ ] 7.3 前回の信号からの試行回数と成功回数を載せる（**成功は試行を超えない**）。
  Scenario: `試行回数と成功回数が載る`。検証: `cargo test attempts_successes` rc=0

## 8. 送信と保持

- [ ] 8.1 追記のみの JSONL で未送信を保持し、到達できたら送る。**上限は置かない**（design D3）。
  Scenario: `到達できない間の記録が後から届く`。検証: `cargo test outbox_survives_outage` rc=0
- [ ] 8.2 1 件ごとの結果の `accepted` だけを見て未送信から取り除く（`docs/collector-contract.md` §返る形）。
  検証: `cargo test outbox_uses_accepted_only` rc=0
- [ ] 8.3 外部サービス上の識別子を付けずに送り、断られないことを確かめる。
  Scenario: `識別子を持たない記録が受け付けられる`。検証: `tools/smoke.sh` rc=0
- [ ] 8.4 1 時間ごとに時計のずれの測定記録を出す（design D6）。検証: `cargo test clock_skew_is_measured` rc=0

## 9. ログと常駐

- [ ] 9.1 ログにアプリ名・ウィンドウ題名・URL・原文を出さない。出すのは件数・ソース名・所要時間・
  エラーの種別だけ。Scenario: `送信の失敗がログに出ても題名と URL は出ない`。
  検証: `cargo test log_has_no_private_content` rc=0
- [ ] 9.2 ログオン時に自動起動し、トレイに常駐する（design D7・**仮**）。
  検証: `crates/collector-windows/README.md` に手順があり、`grep -c "自動起動" ...` が 1 以上

## 10. 仕上げ

- [ ] 10.1 `openspec validate st07-active-window --strict` rc=0
- [ ] 10.2 `python3 scripts/check_scenarios.py .` rc=0（**23 本すべてに印**）
- [ ] 10.3 `python3 scripts/check_chain.py .` rc=0
- [ ] 10.4 `python3 scripts/review_triage.py . st07-active-window` rc=0
- [ ] 10.5 `tools/smoke.sh` rc=0 / `tools/check-immutable.sh` rc=0 / `tools/check-migrations.sh` rc=0
- [ ] 10.6 `cargo test --workspace` rc=0 / `cargo clippy --workspace -- -D warnings` rc=0
- [ ] 10.7 `docs/handoff/` を読み直す（PR 前の 2 回目）

## 人間の確認待ち

**Windows の実環境でしか確かめられないもの。** 確認バッチ（`/verify`）でまとめて見る。

- [ ] V1 Scenario: `アプリを切り替えると 1 件増える`（実機で切り替えて件数を見る）
- [ ] V2 Scenario: `題名が最小滞留より短く変わり続けても記録は増えない`
  （動画を 2 分再生して、件数が再生秒数ぶん増えていないこと）
- [ ] V3 Scenario: `起動時に止まっていた期間が 1 件残る`（PC を落として翌日起動する）
- [ ] V4 Scenario: `離席の始まりと終わりが残る`（5 分以上席を離れて戻る）
- [ ] V5 Scenario: `除外に登録した対象の本文は残らない`（パスワード管理ソフトを登録して開く）
- [ ] V6 Scenario: `表示されている文字列を補正しない`（`https://` が隠れた表示のページを開く）
