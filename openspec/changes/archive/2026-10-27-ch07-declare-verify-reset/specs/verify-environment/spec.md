## ADDED Requirements

### Requirement: 作り直したら、検証の準備は作り直したものを 1 行で宣言する

WHEN 検証の準備（`tools/verify-env.sh`）が `HARNESS_VERIFY=1` で呼ばれ、作り直す木（Requirement「作り直すのは、印があり、worktree か CI のときだけ」）であり、
この worktree の DB のコンテナが使っていた volume を 1 つ以上消したか、DB のコンテナが無くて新しく立て、DB の役割まで整えて rc=0 で終わるとき、
THE SYSTEM SHALL 標準出力に `HARNESS_RESET: testdb` だけの行を、行頭から 1 行だけ出す。DB のコンテナが止まっていた場合も同じである。
行の前に何も付けず、volume の名前や切った接続の数を足さない（それは切った接続の 1 行が持つ）。標準エラーには出さない。
宣言の行は `tools/verify-env.sh` 自身が出し、字 `HARNESS_RESET: testdb` をその本文に置く（ハーネスの点検は準備の本文の字で読む）。
作り直しの対象・手順・切った接続の 1 行・`--plan` の出力は変えない。
導出元: CH07

#### Scenario: 溜まった DB を作り直したら、宣言を 1 行出す
- **WHEN** worktree の DB に `core.event` の行が 3 件ある状態（DB のコンテナは volume を持って動いている）で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、標準出力に `HARNESS_RESET: testdb` だけの行がちょうど 1 つある
- **AND** 標準エラーに `HARNESS_RESET:` で始まる行は無い

#### Scenario: 2 回続けて作り直しても、どちらも宣言を 1 行出す
- **WHEN** `HARNESS_VERIFY=1 tools/verify-env.sh` が作り直しを終えた直後に、もう一度 `HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、2 回目の標準出力にも `HARNESS_RESET: testdb` だけの行がちょうど 1 つある

#### Scenario: DB のコンテナが無くても、立てたら宣言を 1 行出す
- **WHEN** worktree の DB のコンテナとその volume が無い状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、DB のコンテナが立っており、標準出力に `HARNESS_RESET: testdb` だけの行がちょうど 1 つある

#### Scenario: DB のコンテナが止まっていても、作り直したら宣言を 1 行出す
- **WHEN** worktree の DB のコンテナを止めた（volume は持ったまま）状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、出力の 1 行に `切った接続: 0（DB は止まっていた）` がある
- **AND** 標準出力に `HARNESS_RESET: testdb` だけの行がちょうど 1 つある

#### Scenario: ハーネスの点検が、作り直して宣言する準備と認める
- **WHEN** この木で、準備の宣言を確かめる版のハーネスの点検 `python3 .harness2/scripts/init.py --check` を呼ぶ
- **THEN** 出力に `[人間] tools/verify-env.sh` を含む行が無く、`[ok] tools/verify-env.sh` を含む行がちょうど 1 つある

### Requirement: 作り直していなければ、検証の準備は宣言を出さない

THE SYSTEM SHALL 検証の準備が DB の中身を作り直さずに終わるとき、または非 0 で終わるときは、`HARNESS_RESET:` で始まる行を標準出力にも標準エラーにも出さない。
当たるのは、印が無いとき・CI でない main の checkout（worktree でない木）で印があるとき・git の木でないとき・作り直せずに（作り直しの途中で落ちたときを含む）非 0 で終わるとき・
DB のコンテナはあるが volume を 1 つも持たない（消した volume が 0 個の）ときである。`HARNESS_RESET: none` も出さない。
`tools/verify-env.sh --plan` は判定の 1 行（`reset` か `keep: <理由>`）だけを出し、宣言を出さない。
導出元: CH07

#### Scenario: 印が無ければ、宣言を出さない
- **WHEN** worktree の DB に `core.event` の行が 3 件ある状態で、`HARNESS_VERIFY` を渡さずに `tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、`core.event` の行は 3 件のままである
- **AND** 標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: CI でない main の checkout では、印つきでも宣言を出さない
- **WHEN** worktree でない木の DB に `core.event` の行が 3 件ある状態で、`CI` を渡さず `HARNESS_VERIFY=1` で `tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、標準エラーに `keep:` で始まる 1 行がある
- **AND** 標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: 作り直せずに非 0 で終わるときは、宣言を出さない
- **WHEN** DB のコンテナの volume を、compose の外の `ch06-sentinel-` で始まるコンテナにも付けた状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc は 0 でなく、標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: 作り直しの途中で落ちたら、宣言を出さない
- **WHEN** 作り直しの前に DB のコンテナの volume の名前を控え、DB の役割を整える段が失敗する（`ENV_FILE` に無いファイルを指す）ようにして `HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc は 0 でなく、控えた名前の volume は無い（volume を消した後で落ちた）
- **AND** 標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: git の木でなければ、印つきでも作り直さない判定になる
- **WHEN** git の木でない一時の場所に写した `tools/verify-env.sh` を、`CI` を渡さず `HARNESS_VERIFY=1` で `--plan` で呼ぶ
- **THEN** 出力は `keep:` で始まる 1 行で、`HARNESS_RESET:` で始まる行は無い

#### Scenario: volume を持たない DB のコンテナでは、作り直しても宣言を出さない
- **WHEN** worktree の DB のコンテナが volume を 1 つも持たずに動いている状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、出力の 1 行に `切った接続: ` がある（作り直しの経路を通った）
- **AND** 標準出力にも標準エラーにも `HARNESS_RESET:` で始まる行は無い

#### Scenario: 判定だけを出すときは、宣言を出さない
- **WHEN** 一時の repo の main の checkout と worktree で、`HARNESS_VERIFY=1` を渡したときと渡さないときに `tools/verify-env.sh --plan` を呼ぶ
- **THEN** どれも出力は 1 行で、`reset` か `keep:` で始まる行であり、`HARNESS_RESET:` で始まる行は無い
