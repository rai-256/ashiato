## MODIFIED Requirements

### Requirement: 印があれば、検証の準備がこの worktree の DB を空から作り直す

WHEN 検証の準備（`tools/verify-env.sh`）が `HARNESS_VERIFY=1` で呼ばれ、作り直す木（下の Requirement）であるとき、
THE SYSTEM SHALL この worktree の compose project の DB のコンテナとその volume を消して空の DB を立て直し、DB の役割（`tools/db-roles.sh`）を整えてから rc=0 で終わる。
前の DB にあった表・行・database は、立て直した DB に 1 つも残らない。移行は当てない（server の試験の初回が当てる）。
消すのは、この worktree の DB のコンテナが使っていた volume だけで、名前で 1 つずつ消す。立て直した後に前の volume は残らない。
DB の network は消さない。ほかの compose project のコンテナ・volume と、compose の外のコンテナ・volume には触らない。
ただし、作業ツリーが消えたことを機械で確かめられる `<main の名前>-` の compose project のコンテナとネットワークは、作り直しの前に片付けが外す（下の Requirement「作り直す検証の準備は、消えた作業ツリーのコンテナとネットワークを片付け、volume は消さない」）。volume はこのときも消さない。
導出元: CH06、CH05

#### Scenario: 溜まった DB は、印つきの準備の後に空になる
- **WHEN** worktree の DB に `core.event` の行が 3 件と、名前が `ch06_keep_` で始まる database が 1 つある状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、`core.event` は無いか 0 行である
- **AND** `ch06_keep_` で始まる database は無い

#### Scenario: 作り直した後も、所有者とアプリの役割で繋がる
- **WHEN** `HARNESS_VERIFY=1 tools/verify-env.sh` が作り直しを終えた後に、`.env`（か環境変数）の合言葉で `ashiato_owner` と `ashiato_app` が `ashiato` database に繋ぐ
- **THEN** どちらも繋がり、`ashiato` database の所有者は `ashiato_owner` である

#### Scenario: 作り直しの後、前の DB の volume は残らない
- **WHEN** 作り直しの前に DB のコンテナの volume の名前を控え、`HARNESS_VERIFY=1 tools/verify-env.sh` を 2 回続けて呼ぶ
- **THEN** どちらも rc=0 で、控えた名前の volume は `docker volume ls` に無い
- **AND** 立て直した DB のコンテナの volume はちょうど 1 つである

#### Scenario: 作り直しでは、DB の network を手放さない
- **WHEN** 作り直しの前に DB の network の id を控え、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** 作り直しの後の DB の network の id は控えた id と同じである

#### Scenario: ほかの compose project と compose の外のものには触らない
- **WHEN** 別の compose project（名前が `ch06-sentinel-` で始まり、service の名前が `db`）のコンテナと、compose の外の `ch06-sentinel-` で始まるコンテナとそれが使う volume を置いた状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** どちらのコンテナと volume も、作り直しの後に同じ id で残っている

#### Scenario: 消えた作業ツリーのコンテナは、印つきの準備で外れ、その volume は残る
- **WHEN** main の名前を一時の名前に差し替え、project の名前が `<main>-st97`・作業ツリーの path が main の親のフォルダの下の無い path、の印を持つ動いているコンテナ（名前の無い volume つき）を置いた状態で、`HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、そのコンテナは無く、その volume（同じ id）は残っている
- **AND** 標準エラーに `切った接続:` の 1 行がある（この worktree の DB を立て直している）

## ADDED Requirements

### Requirement: 作り直す検証の準備は、消えた作業ツリーのコンテナとネットワークを片付け、volume は消さない

WHEN 検証の準備（`tools/verify-env.sh`）が作り直す（`--plan` が `reset`）とき、
THE SYSTEM SHALL この worktree の DB を消す前に `tools/docker-tidy.sh` を走らせ、作業ツリーが消えたことを機械で確かめられる docker のコンテナとネットワークだけを外す。
片付けは作り直すときだけ呼ぶ（印が無いとき・`--plan` では呼ばない）。
コンテナを外すのは、次を全部満たすときだけとする: compose の project の印と作業ツリーの path の印（`com.docker.compose.project`・`com.docker.compose.project.working_dir`）を持つ /
project の名前が `<main の名前>-` で始まる / 作業ツリーの path の親のフォルダが main の checkout の親のフォルダと同じ / その path がファイルシステムに無く、`git worktree list` にも無い /
その path が main の checkout でも確認バッチの作業ツリー（`-verify` で終わる）でもない。動いているコンテナも、この条件を満たせば止めて外す。
ネットワークを外すのは、compose の project の印を持ち、project の名前が `<main の名前>-` で始まり、繋がっているコンテナが（止まっているものも含めて。`docker ps -a --filter network=` で読む）この回で外すもののほかに無く、その project の印を持つコンテナもこの回で外すもののほかに無く、
`git worktree list` のどの作業ツリーの名前（小文字にしたもの）とも project の名前が同じでなく、確認バッチ（`-verify`）の project でもないときだけとする。コンテナを外した結果で空くネットワークも同じ回で外す。
docker の volume は 1 つも消さない（コンテナは volume ごと消す指定をせずに外す。名前の無い volume はコンテナを外した後も残る）。印の無いコンテナ・ネットワーク、別の repo の compose の project には触らない。
外したコンテナとネットワークは、1 つごとに名前と、コンテナならその作業ツリーの path を標準エラーに 1 行出す。外すものが無ければ何も出さない。`--dry-run` は外すもの（コンテナを外した結果で空くネットワークを含む）を同じ形で出すだけで、何も外さない。
外そうとしたものがもう無い（並んで走った別の片付けが先に外した）ときは、外せたものとして扱う。
片付けが docker に問えない・外すのに失敗したときは、理由を標準エラーに出して rc≠0 で終わり、検証の準備は `片付けに失敗した` と片付けの rc を含む 1 行を標準エラーに出したうえで作り直しを続ける（片付けが落ちても、この worktree の検証は止めない）。片付け自身の出す行は `片付けに失敗した` を含まない。
同じ検証の準備のほかの Requirement が言う「何も消さずに非 0 で終わる」（作り直せないとき・subnet を塞ぐ network にコンテナが繋がっているとき）は、この worktree の DB のコンテナ・volume と、塞いでいる network を指す。片付けはその判定より前に終わり、外したものはその判定が非 0 になっても戻さない。
CI の rust job は片付けの `--self-test` を走らせる。
導出元: CH05

#### Scenario: 作業ツリーが消えたコンテナは、動いていても外される
- **WHEN** main の名前が `<main>` の木で、project の名前が `<main>-st97`・作業ツリーの path が main の親のフォルダの下の無い path、の印を持つ動いているコンテナと、その project のネットワークがある状態で片付けを走らせる
- **THEN** 片付けは rc=0 で終わり、そのコンテナとそのネットワークは無い
- **AND** 標準エラーに、外したコンテナの名前と作業ツリーの path、外したネットワークの名前が出る

#### Scenario: 外したコンテナの名前の無い volume は残る
- **WHEN** 作業ツリーが消えたコンテナが名前の無い volume を持つ状態で片付けを走らせる
- **THEN** そのコンテナは無く、その volume（同じ id）は残っている

#### Scenario: 作業ツリーが残っていれば、止まっているコンテナも外さない
- **WHEN** `git worktree list` にあり path もある作業ツリーの path の印を持つ止まっているコンテナと、その project のネットワークと、path はあるが `git worktree list` に無い path の印のコンテナと、`git worktree list` にあるが path が無い（prune の前の）path の印のコンテナがある状態で片付けを走らせる
- **THEN** 片付けは rc=0 で終わり、3 つのコンテナとそのネットワークは残る

#### Scenario: main の checkout と確認バッチのコンテナは、path が無くても外さない
- **WHEN** 作業ツリーの path の印が main の checkout の path のコンテナと、`<main>-verify` の path のコンテナ（どちらの path もファイルシステムに無いとして渡す）がある状態で片付けを走らせる
- **THEN** 片付けは rc=0 で終わり、どちらのコンテナも残る

#### Scenario: 印の無いコンテナと、別の repo の compose のコンテナには触らない
- **WHEN** compose の印の無いコンテナと、project の名前が `<main>-` で始まらない（`ch06-sentinel-` で始まるものを含む。または作業ツリーの path の親のフォルダが main と違う）、path の無いコンテナがある状態で片付けを走らせる
- **THEN** 片付けは rc=0 で終わり、どのコンテナも残る

#### Scenario: コンテナの無いネットワークは、作業ツリーが残っていなければ外す
- **WHEN** コンテナが 1 つも繋がっていない `<main>-st98` の project のネットワークと、コンテナが 1 つも繋がっていないが `git worktree list` にある作業ツリーの project のネットワークと、確認バッチの project のネットワークと、
  外さないコンテナが繋がった `<main>-` のネットワークと、外さない止まったコンテナだけが繋がった `<main>-` のネットワークと、コンテナは繋がっていないがその project の印を持つ外さないコンテナがある `<main>-` のネットワークと、印の無いネットワークと、project の名前が `<main>-` で始まらないネットワークがある状態で片付けを走らせる
- **THEN** 片付けは rc=0 で終わり、前のネットワークは無く、後の 7 つは残る

#### Scenario: --dry-run は外すものを出すだけで、何も外さない
- **WHEN** 作業ツリーが消えたコンテナと、そのコンテナだけが繋がったネットワークと、コンテナの無い消えた作業ツリーのネットワークがある状態で、片付けを `--dry-run` で走らせる
- **THEN** 片付けは rc=0 で終わり、標準エラーにそのコンテナと 2 つのネットワークの名前が出る
- **AND** そのコンテナとネットワークは残っている

#### Scenario: 外すものが無ければ、片付けは何も出さない
- **WHEN** 外す条件に当たるコンテナもネットワークも無い状態で片付けを走らせる
- **THEN** 片付けは rc=0 で終わり、標準エラーにも標準出力にも何も出ない

#### Scenario: もう無いものを外そうとしたら、外せたものとして扱う
- **WHEN** 外すコンテナとネットワークに対して docker が `No such container` / `No such network` を返す状態で片付けを走らせる
- **THEN** 片付けは rc=0 で終わる

#### Scenario: 片付けが docker に問えなければ、片付けは rc≠0 で落ちる
- **WHEN** 片付けが docker に問う命令を、必ず失敗する命令に替えて片付けを走らせる
- **THEN** 片付けは rc≠0 で終わり、標準エラーに理由が出る

#### Scenario: 外す命令が失敗すれば、片付けは rc≠0 で落ちる
- **WHEN** 外すコンテナがある状態で、外す命令だけが（もう無い以外の理由で）失敗する命令に替えて片付けを走らせる
- **THEN** 片付けは rc≠0 で終わり、標準エラーに外せなかったものの名前と理由が出る

#### Scenario: 片付けが落ちても、検証の準備は DB を作り直して続く
- **WHEN** 片付けが docker に問う命令を、必ず失敗する命令に替えて `HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ
- **THEN** rc=0 で、標準エラーに `片付けに失敗した` と片付けの rc を含む 1 行と、`切った接続:` の 1 行がある
- **AND** この worktree の DB のコンテナが立っている

#### Scenario: 印の無い検証の準備と --plan は、片付けを呼ばない
- **WHEN** 片付けが docker を呼ぶ命令を、打たれた引数を書き留める偽の命令に替えて、`HARNESS_VERIFY=1 tools/verify-env.sh`（作り直す）を呼んで書き留めた命令を数え直し、続けて `HARNESS_VERIFY` を渡さずに `tools/verify-env.sh` と `HARNESS_VERIFY=1 tools/verify-env.sh --plan` を呼ぶ
- **THEN** 作り直す呼び出しでは書き留めた命令が 1 つ以上あり、後の 2 つはどちらも rc=0 で、新しく書き留めた命令は 1 つも無い

#### Scenario: 片付けが docker に打つ命令に、volume を消すものは無い
- **WHEN** 作業ツリーが消えたコンテナとネットワークがある状態で、docker を呼ぶ命令を、打たれた引数を書き留める偽の命令に替えて片付けを走らせる
- **THEN** 書き留めた命令の subcommand は `ps`・`inspect`・`network ls`・`network inspect`・`rm`・`network rm` だけで、`rm` の引数は `-f` とコンテナの id だけである（`-v`・`--volumes`・`-fv` などは無い）

#### Scenario: CI の rust job は、片付けの self-test を走らせる
- **WHEN** `.github/workflows/ci.yml` の rust job を読む
- **THEN** `tools/docker-tidy.sh --self-test` を走らせる
- **AND** 検証の準備への組み込みを確かめる `tools/docker-tidy.sh --verify-env-check` も走らせる
