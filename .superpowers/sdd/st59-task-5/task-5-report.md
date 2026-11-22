# Task 5 実装報告

## 実装したこと

- `tools/settings-server.sh` を追加した。
- `make <置き場>` で、試験用の `settings/server.json` と元の固定合言葉を読む別ファイルを作る。
- `up <設定のファイル>` で設定を環境へ反映し、`WEB_PASSWORD`・管理者用・所有者用の環境変数を外して、サービス形のサーバを `ASHIATO_HTTP_PORT + 1` で起動する。画面を組み込んだ release binary が無い場合は `stack.sh` と同様に build する。
- `down`（設定ファイル指定時はその pid file）と、DB を使わない `--self-test` を実装した。

## TDD Evidence

### RED

実装前に次を実行し、未実装のため rc=127 で失敗した。

```text
tools/settings-server.sh --self-test
rc=127: tools/settings-server.sh: No such file or directory
```

### GREEN

実装後、次が成功した。

```text
tools/settings-server.sh --self-test
self-test 通過（設定の鍵と起動環境の名前を確認した）
```

さらに `make` の JSON 鍵一覧、別ファイル生成、標準出力に固定合言葉・偽の API 値が出ないことを確認した。

## 検証

- `scripts/verify-run 5.1` — PASS（`tools/settings-server.sh --self-test` rc=0、`tools/check-secrets.sh` rc=0）
- `bash -n tools/settings-server.sh` — PASS
- `git diff --check` — PASS

## 変更ファイル

- `tools/settings-server.sh`
- `openspec/changes/st59-change-password-from-web/evidence.jsonl`

## 自己レビュー

設定値や合言葉は標準出力・起動ログへ出さず、サーバ子プロセスには元の `WEB_PASSWORD` と管理者・所有者用の秘密を渡さないことを確認した。Task 5 に Scenario の名指しは無いため、Scenario 印の追加はない。

懸念はない。
