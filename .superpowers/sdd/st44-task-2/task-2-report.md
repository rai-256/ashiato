# Task 2 report

## 実装

- `ApiDoc::openapi()` の `paths` と HTTP method から operation 一覧を作る test helper を追加した。
- `{sha256}` / `{id}` / `{cid}` を形に合う値で埋め、router 経由で全 operation を呼ぶ試験を追加した。
- 資格情報なしでは `/healthz` が 200、`DELETE /session` が 204、`POST /session` とその他が 401 になることを確認する `st44_routes_without_credentials` を追加した。
- API の合言葉で許可一覧外をすべて 403 とし、各 route の拒否ログ、保護対象テーブル（event・滞在・場所・属性・衛星・書き出し）の行数不変を確認する `st44_routes_api_token_outside_403` を追加した。
- `COLLECTOR_ALLOWED` の 9 operation が契約に載ることを確認する `st44_routes_allowlist_in_contract` を追加した。
- 衛星側の `is_exempt` を共有して、資格情報不要の 3 route が両試験で一致することを確認した。

## テスト

- RED: `scripts/quiet-run red -- cargo test -p ashiato-server st44_routes_`。初回は追加テストの型不一致・所有権エラーで rc=101。エラーを修正後、テストが実行できる状態にした。
- GREEN: `scripts/quiet-run ct-final -- cargo test -p ashiato-server st44_routes_`。3/3 PASS。
- `scripts/quiet-run sat-final -- cargo test -p ashiato-server every_route_refuses_satellite`。1/1 PASS。
- `-- --list` の件数は `st44_routes_` が 3、`every_route_refuses_satellite` が 1。
- `cargo fmt --check`、`git diff --check` PASS。
- 並列の全体実行は 1871/1872 PASS。追加試験が共有 DB の並列更新 3 行を行数不変違反として検出したため rc=101 になった。
- 直列の全体実行 `scripts/quiet-run full-server-serial -- cargo test -p ashiato-server -- --test-threads=1` は `1872 passed, 0 failed, 3 ignored`、起動試験は `37 passed`。

## 変更ファイル

- `crates/server/src/collector_scope_tests.rs`
- `crates/server/src/satellite_routes_tests.rs`

## 自己レビュー

OpenAPI から operation を動的に取得し、router と middleware を通して、path parameter を型に合わせて埋めている。資格情報なしのリクエストは引数・本文を最小化し、認証前に 401 になることを検査している。並列全体実行では共有 DB の他試験による行数変化があるため失敗したが、対象試験単独および全体直列実行では再現せず PASS した。

## 懸念

並列実行時の共有 DB 行数不変 assertion は、他の DB 試験が同時に行を追加すると競合する。Task の要求する「呼ぶ前後の行数不変」を保持するため assertion は削っていない。ハーネスの Task 検証コマンドは対象試験単独なので PASS する。

## Task gate 証跡

`scripts/verify-run 2.1` を実行し、指定された 2 つの検証コマンドがいずれも rc=0 / PASS になった。証跡は `openspec/changes/st44-collector-credential-scope/evidence.jsonl` に記録されている。

## Fix round 1

### 対応

- F1: `st44_routes_api_token_outside_403` だけを `testdb::isolated()` の DB 上で組み立てる `isolated_app` に切り替え、並列実行時に他試験の共有 DB 更新が行数比較へ混入しないようにした。
- F2: 拒否ログの行数が拒否した口数と一致する `rows.len() == called.len()` の検証を追加した。

### 検証

- `scripts/quiet-run fix-focused -- cargo test -p ashiato-server st44_routes_`: rc=0。ログ内で 3/3 PASS。
- `scripts/verify-run 2.1`: 2 件とも PASS。st44_routes_ は 3 本、every_route_refuses_satellite は 1 本を実行。
- `scripts/quiet-run full-server-fix -- cargo test -p ashiato-server`: rc=0。1872 passed, 0 failed, 3 ignored、および起動試験 37 passed。
- `HX_CHANGE=st44-collector-credential-scope python3 scripts/check_scenarios.py .`: 今回追加した 2 印は認識されたが、change 全体の既存未担保 Scenario 5 件で FAIL。今回の修正範囲外。

### 自己レビュー

F1 の DB 隔離用 `TestDb` は戻り値で保持し、App のプールが使用中に破棄されないことを確認した。F2 は集合の一致に加えて行数を比較するため、二重記録を検出できる。追加の懸念はない。
