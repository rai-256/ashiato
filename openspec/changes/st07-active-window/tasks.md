# ST07 実装タスク — PC のアクティブウィンドウを集める

読む順: `deep.md`（**最優先。本人が決めた 8 件**）→ このファイル → `specs/desktop-collection/spec.md`
→ `design.md` → `docs/stories/ST07.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**この change は移行を 1 本も足さない**（design D2）。足す必要が出たら
**名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
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

## 1. 登録簿の担保

**移行は 1 本も足さない**（design D2）。`c02-window` は登録済みで、`external_id_kind` も
ST03 の移行（`202609120940_source_columns.sql`）が既に `'none'` にしている（2026-09-13 に実測）。
残すのは、**誰が倒したかに依存しない**担保 1 本だけ。

- [x] 1.1 全移行を当てた後に `c02-window` の `external_id_kind` が `'record'` ではないことを見る
  テストを 1 本置く（design D2）。**列の有無で分岐させない** —— 分岐すると「まだ列が無いから合格」で
  素通りする。検証: `cargo test c02_window_external_id_kind_is_not_record` rc=0

## 2. 送る形（契約）

- [x] 2.1 `docs/collector-contract.md` に **C-02 が送る `payload` の形**を追記する（design D1）——
  アプリ名（表示名）・実行ファイルのパス・プロセス名・ウィンドウ題名・URL・記録の種類
  （`foreground` / `idle` / `powered-off` / `excluded`）・範囲の終わり（`idle` と `powered-off` のみ）・
  **最後の入力からの経過時間**（`idle` のみ。design D9）・**除外した件数**（`excluded` のみ。design D11）。
  **`raw` は収集側が組んだ JSON を文字列のまま送る**ことを明記する。
  検証: `grep -c "c02-window" docs/collector-contract.md` が 1 以上、
  `python3 scripts/check_chain.py .` rc=0

## 3. 前景の変化を拾う

- [x] 3.1 `crates/collector-windows` に前景の読み取りを実装する。OS のイベント通知と
  1 秒間隔の見回りを併用する（design D5・**仮**）。検証:
  `cargo test --package ashiato-collector-windows foreground` rc=0
- [x] 3.2 アプリ名・ウィンドウ題名・URL のいずれかが変化したら記録を 1 件作る。
  Scenario: `アプリを切り替えると 1 件増える` / `同じアプリの中で題名が変われば 1 件増える` /
  `URL だけが変われば 1 件増える`。検証: `cargo test --package ashiato-collector-windows` rc=0
- [x] 3.3 URL は**前景がブラウザのときだけ** UI Automation でアドレスバーを読む（design D4 / Risks）。
  **補正しない**（design D12）。Scenario: `クエリとフラグメントが残る` / `表示されている文字列を補正しない`。
  検証: `cargo test url_is_not_normalized` rc=0
- [x] 3.4 題名だけの変化に最小滞留 5 秒を置く。**アプリと URL の変化は滞留に関わらず必ず 1 件**
  （design D8・**仮**）。Scenario: `題名が最小滞留より短く変わり続けても記録は増えない` /
  `アプリの切り替えは滞留時間に関わらず記録される` / `URL の変化は滞留時間に関わらず記録される`。
  検証: `cargo test min_dwell` rc=0

## 4. 離席（FR-81）

- [x] 4.1 最後の入力からの経過時間・画面ロック・スリープを読む。閾値は 5 分（design D9・**仮**）。
  **経過時間も記録に載せる**（閾値を変えたときに引き直せるようにするため）。
  Scenario: `離席の始まりと終わりが残る` / `画面ロックとスリープも残る`。
  検証: `cargo test idle_transitions` rc=0
- [x] 4.2 離席の記録を前景の記録と区別できる形にする（`payload` の記録の種類。design D1）。
  Scenario: `離席の記録は前景の記録と区別できる`。検証: `cargo test record_kind_is_distinguishable` rc=0

## 5. PC が止まっていた期間（FR-82）

- [x] 5.1 停止時刻の印をローカルに書き、**動作中は 1 分ごとに更新する**（design D10 / Risks）。
  検証: `cargo test last_seen_marker` rc=0
- [x] 5.2 起動時に、印と現在時刻から区間の記録を 1 件作る。**印が無ければ書かない**。
  Scenario: `起動時に止まっていた期間が 1 件残る` / `初回起動では生成しない`。
  検証: `cargo test powered_off_span` rc=0
- [x] 5.3 この記録が**意図的な停止（FR-34）と区別できる**ことを固定する（design D10）。
  Scenario: `意図的な停止と区別できる`。検証: `cargo test powered_off_is_not_stopped` rc=0

## 6. 除外（FR-83）

- [x] 6.1 除外する対象の登録の形（実行ファイルのパス / プロセス名 / ウィンドウ題名の一致規則）を作る。
  **既定は空**。検証: `cargo test exclusion_empty_by_default` rc=0
- [x] 6.2 除外の判定を**送る前**に行い、本文を持たない記録を 1 件書く（design D11）。
  Scenario: `除外に登録した対象の本文は残らない` / `除外した件数が残る` / `除外の登録が空なら何も除外されない`。
  検証: `cargo test exclusion` rc=0
- [x] 6.3 直列化の形を Rust 側のテストで固定する（**形が変わると同じ 1 件が別の鍵になる**。design D1）。
  **4 章・6 章で項目が出そろってから凍結する** —— 先に凍結すると 2 回書き換えることになり、
  形が変わるたびに同じ 1 件が別の鍵になる（spec-review R6）。
  検証: `cargo test payload_shape_is_pinned` rc=0
- [x] 6.4 **除外リストの初期登録の手順**を `crates/collector-windows/README.md` に書く
  （design Risks の「URL にトークンが入る」の守りがこれと PERM-9 の 2 つしかないため）。
  検証: `grep -c "除外" crates/collector-windows/README.md` が 1 以上

## 7. 生存信号（FR-78）

- [x] 7.1 想定間隔（登録簿の `c02-window` = 21600 秒）ごとに生存信号を送る。
  **注入した時計で間隔を進める単体テストで判定する** —— smoke は数十秒で終わるので、
  間隔を守らない実装でも必ず緑になる（spec-review R8）。
  Scenario: `想定間隔ごとに生存信号が届く`。
  検証: `cargo test heartbeat_interval_is_expected_gap` rc=0。
  併せて `tools/smoke.sh` に「生存信号が 1 件届く」手順を足して rc=0
- [x] 7.2 取得可否を**前景ウィンドウが取れること**と **UI Automation が応答すること**で判定し、
  満たされていない側を `blockers` に載せる（design D4）。
  Scenario: `前景を読めない状態は取得できないとして報告される`。検証: `cargo test capturable_blockers` rc=0
- [x] 7.3 前回の信号からの試行回数と成功回数を載せる（**成功は試行を超えない**）。
  Scenario: `試行回数と成功回数が載る`。検証: `cargo test attempts_successes` rc=0

## 8. 送信と保持

- [x] 8.1 追記のみの JSONL で未送信を保持し、到達できたら送る。**上限は置かない**（design D3）。
  Scenario: `到達できない間の記録が後から届く`。検証: `cargo test outbox_survives_outage` rc=0
- [x] 8.2 1 件ごとの結果の `accepted` だけを見て未送信から取り除く（`docs/collector-contract.md` §返る形）。
  検証: `cargo test outbox_uses_accepted_only` rc=0
- [x] 8.3 外部サービス上の識別子を付けずに送り、断られないことを確かめる。
  **`external_id` は null で送る（空文字は不可）** —— ST03 が `empty_external_id` を足しており、
  空文字は 400 で断られる（`docs/collector-contract.md` §返る形）。
  `source_updated_at` と `external_ref` はどちらも省略可なので送らない。
  Scenario: `識別子を持たない記録が受け付けられる`。検証: `tools/smoke.sh` rc=0

## 8b. 感度と時計

- [x] 8b.1 **記録に感度を明示せず、既定（`sensitivity=1` = 外部 AI に出してよい）に委ねる**
  （深掘り Q3。**本人が推奨と違う側を選んだ唯一の決定**）。
  **「題名と URL は私的だから」と厳しい側に倒さない。**
  Scenario: `既定の感度で格納される` / `収集側が厳しい側の感度を付けて送らない`。
  検証: `cargo test sensitivity_uses_collection_default` rc=0。
  併せて `tools/smoke.sh` で `psql -c "SELECT sensitivity FROM core.event WHERE
  logical_source='c02-window' LIMIT 1"` が `1` を返す
- [x] 8b.2 1 時間ごとに時計のずれの測定記録を出す（design D6）。
  Scenario: `1 時間ごとにずれの測定記録が残る`。検証: `cargo test clock_skew_is_measured` rc=0

## 9. ログと常駐

- [x] 9.1 ログにアプリ名・ウィンドウ題名・URL・原文を出さない。出すのは件数・ソース名・所要時間・
  エラーの種別だけ。Scenario: `送信の失敗がログに出ても題名と URL は出ない`。
  検証: `cargo test log_has_no_private_content` rc=0
- [x] 9.2 ログオン時に自動起動する（design D7・**仮**）。スタートアップフォルダへ
  `ashiato-collector.cmd` を 1 つ置く（`--install-autostart`。レジストリは別の crate か
  `unsafe` を要するので採らない。design D15）。
  検証: `grep -c "自動起動" crates/collector-windows/README.md` が 1 以上 / `cargo test startup_script` rc=0
- [ ] 9.3 **トレイに常駐して「動いている」ことを見せる**（design D7 の後半。**残した**）。
  理由: 窓とメッセージの輪を持つことになり、見回りの作り（`runtime::tick`）が変わる。
  止まっていることに気づく手段は当面 生存信号と稼働状況の画面（FR-78 / FR-80）。
  検証: 実機でトレイに出ることを見る（確認バッチ）

## 10. 仕上げ

- [x] 10.1 `openspec validate st07-active-window --strict` rc=0
- [x] 10.2 `python3 scripts/check_scenarios.py .` rc=0（**23 本すべてに印**）
- [x] 10.3 `python3 scripts/check_chain.py .` rc=0
- [x] 10.4 `python3 scripts/review_triage.py . st07-active-window` rc=0
- [x] 10.5 `tools/smoke.sh` rc=0 / `tools/check-immutable.sh` rc=0 / `tools/check-migrations.sh` rc=0
- [x] 10.6 `cargo test --workspace` rc=0 / `cargo clippy --workspace -- -D warnings` rc=0
- [x] 10.7 `docs/handoff/` を読み直す（PR 前の 2 回目）

## 人間の確認待ち

**Windows の実環境でしか確かめられないもの。** 確認バッチ（`/verify`）でまとめて見る。
**書式は `- Scenario: <名前>` の裸の形**（チェックボックスも番号も注釈も付けない）——
`check_scenarios.py` / `verify_checklist.py` / `verify_record.py` の 3 本ともこの形しか読まない
（spec-review R3）。やり方は次の行の引用に置く。

- Scenario: アプリを切り替えると 1 件増える
  > 実機でアプリを切り替え、`c02-window` の件数が 1 増えることを見る
- Scenario: 題名が最小滞留より短く変わり続けても記録は増えない
  > 動画を 2 分再生し、件数が再生秒数ぶん増えていないことを見る
- Scenario: 起動時に止まっていた期間が 1 件残る
  > PC を落として翌日起動し、その期間の記録が 1 件あることを見る
- Scenario: 離席の始まりと終わりが残る
  > 5 分以上席を離れて戻り、出入りが 2 件残ることを見る
- Scenario: 除外に登録した対象の本文は残らない
  > パスワード管理ソフトを除外に登録して開き、題名も URL も残っていないことを見る
- Scenario: 表示されている文字列を補正しない
  > `https://` が隠れた表示のページを開き、記録の URL に `https://` が補われていないことを見る
