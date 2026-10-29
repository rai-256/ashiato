## ADDED Requirements

### Requirement: S-01 の版が変わると、更新の履歴に 1 行残る

WHEN `ashiato-server migrate` が版の検査（知らない版・読めない版の記録で止まる検査）を通った THE SYSTEM SHALL 当てる前の退避を取る前に、自分の組み立ての印が更新の履歴の直前の**通った**行の今の版と違えば、移行の結果 `attempted`（試み）の 1 行を書き、退避を取る前に commit する（退避に入るので、退避から D-01 を戻しても残る）。
WHEN `ashiato-server migrate` が終わる（当てた・当てる版が無かった・版の中身が当たらずに落ちた・退避に失敗した）THE SYSTEM SHALL 試みの行を書いた回は比べ直さずに必ず、試みの行を書けなかった回（表がまだ無かった）は自分の組み立ての印が更新の履歴の直前の**通った**行の今の版と違えば、更新の履歴に結果の 1 行を書く。表があって試みの行を書かなかった回（直前の通った行と同じ組み立て）は、結果の行も書かない。
THE SYSTEM SHALL 試みと結果の行を当てる処理の錠を持つ接続で書き、結果の行を錠を外す前に書く。
THE SYSTEM SHALL 起動の行を比べて書く取引では、当てる処理の錠を待たずに取ろうとし、取れなければ（migrate が走っている）起動の行を書かずに種別 `update_history_unwritten` の警告を示して起動を続け、取れたらその中で版の記録をもう一度読み、自分の知らない版があれば起動の行を書かない（試みと結果の間に通った行がほかの書き手から入らず、結果の行の前の版は試みの行の前の版と同じになる。版の検査の後に当たった版の上に、古い組み立ての起動の行を書かない）。
WHEN S-01 が起動し、版の検査（「サーバは DB と版が食い違えば起動しない」）を通った THE SYSTEM SHALL 待ち受けを開く前に、自分の組み立ての印が直前の通った行の今の版と違えば、更新の履歴に 1 行を書く。
THE SYSTEM SHALL 1 行に、書いた時刻（S-01 の時計）・部品（`S-01`）・きっかけ（`migrate` か `startup`）・前の版・今の版・移行の結果を持たせる。
移行の結果は `attempted`（版の検査を通り、退避を取る前に書いた試み。当てた版・落ちた版・SQLSTATE は空）・`applied`（この回に当てた版の名前を `migrations/ORDER` の順に持つ）・`nothing_to_apply`（当てる版が無かった。起動の行はいつもこれ）・`failed`（落ちた版の名前と SQLSTATE を持つ。落ちる前に当て終えた版の名前も持つ）・
`backup_failed`（当てる前の退避に失敗した。版は 1 つも当てていない）のどれかにする。
THE SYSTEM SHALL 「前の版」を、更新の履歴の中で書いた順に最後の、移行の結果が `applied` か `nothing_to_apply` の行の今の版にする（試みの行と落ちた行の今の版は D-01 を動かしていないので数えない）。そういう行が無ければ前の版を空（記録の始まり）にする。
THE SYSTEM SHALL migrate が版の中身の失敗でなく落ちた（当てた後の付与・錠・版の記録の読み書きの失敗。終了コード 1）ときも、表を書けるなら移行の結果 `failed` で 1 行を書き、落ちた版の名前を空、SQLSTATE を取れればその符号にする（D-01 が新しい版へ動いた後に落ちた回を `applied` に見せない）。
THE SYSTEM SHALL 自分の組み立ての印を、`<版番号>+unknown…` の印も含めて、そのまま今の版に書く。
WHILE 2 つの書き手（migrate と起動、2 つの起動、または 2 つの migrate）が同時に比べて書く THE SYSTEM SHALL 同じ版の変化を、migrate なら試みと結果の 1 組、起動なら 1 行だけ書く（題の「1 行」は、版の変化 1 回につき結果の行が 1 つ残ること。migrate はその前に試みの行を 1 つ持つ）。
THE SYSTEM SHALL DB の誤りの本文（行の値を含みうる）を、更新の履歴にも、S-01 のログにも、migrate の標準出力・標準エラーにも残さない。migrate が落ちたときに示すのは、種別・落ちた版の名前・SQLSTATE だけにする。
IF migrate が終了コード 3 で止まる（`db_ahead`・`ledger_unreadable`）、または起動が版の検査で断られる THEN THE SYSTEM SHALL 更新の履歴に何も書かない。
THE SYSTEM SHALL 試みの行の後に、同じ migrate の結果の行（`via` が `migrate` で移行の結果が `attempted` でない行）が、次の試みの行より前に無いことで、その回が結果を書く前に終わった（退避から D-01 を戻した・途中で止まった）と読めるようにする。
IF 更新の履歴の表がまだ無い D-01（ST51 の移行より前で落ちた・退避の失敗・ST51 の移行を当てる前の試み）THEN THE SYSTEM SHALL 行を書かずに、種別 `update_history_unwritten` の警告を示し、migrate の終了コードを変えない。
THE SYSTEM SHALL 更新の履歴を書けなかったことで、migrate の終了コードも起動の成否も変えない。

導出元: FR-121。扉 #33。深掘り（ST51）Q1・Q6。premises の C2・C4・C5（design D1（仮）・D2（仮）・D3（仮）・D7・D8・D9・D16（仮））。review/spec.md R2・R6・R9。
試みの行は、退避から D-01 を戻すと消える結果の行の代わりに、通らなかった回を戻しの後にも 1 行残すため（FR-121 ★ 2026-10-29「退避からの戻しで消えない置き場に書く」・Q6）。落ちた版の名前と SQLSTATE は結果の行にだけあり、戻すと消える —— 戻した後に書き直すのは ST58（Q2）。
起動の行は版の検査の後に書く —— 前に書くと、古い S-01 が新しい D-01 に書くことになる（FR-117）。起動できた S-01 は自分の知る版が全部当たっている（ST48）ので、起動の行の移行の結果は `nothing_to_apply` になる。
誤りの本文を残さないのは、制約に反した誤りが行の値（`Key (x)=(値) already exists`）を含み、追記のみの行と書き出しから消す口が無いから（Q1）。SQLSTATE（5 文字の符号）は値を含まない。

#### Scenario: 新しい組み立ての migrate が版を当てると、当てた版の名前を持つ 1 行が残る

- **GIVEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせた後、試験用の版を 1 つ並びの末尾に足した D-01
- **WHEN** 印 `0.1.0+<commit B>` の組み立てで migrate を走らせる
- **THEN** 終了コードは 0 で、更新の履歴の最後の行は部品 `S-01`・きっかけ `migrate`・前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `applied`・当てた版の名前は足した版の 1 つだけ
- **AND** その 1 つ前の行は、きっかけ `migrate`・前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `attempted`・当てた版の名前と落ちた版の名前と SQLSTATE は空

#### Scenario: 当てる版が無い入れ替えの migrate も 1 行残る

- **GIVEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせた D-01
- **WHEN** 同じ版の並びを持つ印 `0.1.0+<commit B>` の組み立てで migrate を走らせる
- **THEN** 更新の履歴の最後の行は前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `nothing_to_apply`・当てた版の名前は空
- **AND** その 1 つ前の行は今の版 `0.1.0+<commit B>`・移行の結果 `attempted` の行

#### Scenario: 同じ組み立てで migrate を走らせ直しても、行は増えない

- **GIVEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせた D-01
- **WHEN** 同じ組み立てで migrate をもう一度走らせる
- **THEN** 更新の履歴の行の数は走らせる前と同じ

#### Scenario: migrate を走らせずに入れ替えた S-01 は、起動のときに 1 行を書く

- **GIVEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせた D-01
- **WHEN** 同じ版の並びを持つ印 `0.1.0+<commit B>` の組み立ての S-01 を、migrate を走らせずに起動する
- **THEN** `/healthz` が 200 を返し、更新の履歴の最後の行はきっかけ `startup`・前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `nothing_to_apply`

#### Scenario: migrate の後に同じ組み立ての S-01 を起動しても、行は増えない

- **GIVEN** 印 `0.1.0+<commit B>` の組み立てで migrate を走らせた D-01
- **WHEN** 同じ組み立ての S-01 を起動し、止めて、もう一度起動する
- **THEN** 更新の履歴の行の数は migrate の後と同じ

#### Scenario: 移行が落ちた回は、落ちた版と SQLSTATE を持つ 1 行が残り、誤りの本文は残らない

- **GIVEN** 印 `0.1.0+<commit A>` で当てた後、試験用の表の text の列に値 `st51-secret-value` の行を入れ、その列を integer に変えて必ず落ちる試験用の版を並びの末尾に足した D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して、その並びを当てる処理を走らせる
- **THEN** 当てる処理は失敗を返し、更新の履歴の最後の行は移行の結果 `failed`・落ちた版の名前はその版・SQLSTATE は `22P02`・前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`
- **AND** 更新の履歴のどの列にも `st51-secret-value` の文字列が無い

#### Scenario: 移行が落ちた migrate は終了コード 1 で終わり、標準エラーに行の値を出さない

- **GIVEN** 版の記録から 1 行を消し、その版を当て直すと、誤りの本文に行の値 `st51-secret-value` を含む誤り（型の入力の誤りなど）で落ちるように DB の中の物を書き換えた D-01
- **WHEN** `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 1 で、標準出力と標準エラーのどこにも `st51-secret-value` が無く、更新の履歴の最後の行は移行の結果 `failed` で落ちた版の名前はその版

#### Scenario: 落ちた回の後の行の前の版は、最後に通った版になる

- **GIVEN** 印 `0.1.0+<commit A>` で通った後、印 `0.1.0+<commit B>` の migrate が落ちた D-01
- **WHEN** 落ちる版を取り除いた印 `0.1.0+<commit C>` の組み立てで migrate を走らせる
- **THEN** 更新の履歴の最後の行は前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit C>`・移行の結果 `nothing_to_apply`

#### Scenario: 退避に失敗した回は、版を当てていない行が残る

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01 と、退避の道具が失敗する設定
- **WHEN** 退避を有効にして `ashiato-server migrate` を走らせる
- **THEN** 終了コードは 1 で、更新の履歴の最後の行は移行の結果 `backup_failed`・当てた版の名前は空・落ちた版の名前と SQLSTATE は空
- **AND** その 1 つ前の行は今の版が同じ印の、移行の結果 `attempted` の行

#### Scenario: 退避を取るときには、試みの行がもう D-01 に入っている

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して当てる処理を走らせ、退避を取る所で、別の接続から更新の履歴を読む
- **THEN** 別の接続から、今の版 `0.1.0+<commit B>`・移行の結果 `attempted` の行が読める

#### Scenario: 当てる処理の途中で同じ組み立ての S-01 が起動しても、試みと結果の組は崩れない

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して当てる処理を走らせ、退避を取る所で、印 `0.1.0+<commit B>` の起動の書き込みを別の接続から走らせる
- **THEN** 起動の書き込みは待たずに終わって種別 `update_history_unwritten` の警告を示し、更新の履歴の行は書いた順に、試み（前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`）・結果（前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `nothing_to_apply`）の 2 行で、起動の行は無い

#### Scenario: 版の検査の後に知らない版が当たった D-01 には、起動の行を書かない

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置き、版の記録に自分の知らない版の行が 1 つある D-01
- **WHEN** 印 `0.1.0+<commit C>` の起動の書き込みを、版の検査を通さずに直接走らせる
- **THEN** 更新の履歴の行は走らせる前と同じ

#### Scenario: 同じ組み立ての当てる処理を 2 つ同時に走らせても、試みと結果は 1 組だけ

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して当てる処理を 2 つ同時に走らせる
- **THEN** 今の版 `0.1.0+<commit B>` の行は、試みと結果が続けて 1 組だけ

#### Scenario: 更新の履歴の表が無い D-01 に初めて当てて通った migrate は、警告を出し、結果の 1 行だけを残す

- **GIVEN** 何も当たっていない空の DB
- **WHEN** 印 `0.1.0+<commit A>` を渡して全部の版を当てる処理を走らせる
- **THEN** 当てる処理は通り、種別 `update_history_unwritten` の警告が出て、更新の履歴の行は前の版が空・今の版 `0.1.0+<commit A>`・移行の結果 `applied` の 1 行だけ

#### Scenario: 移行が落ちて退避から D-01 を戻しても、通らなかった回の試みの行が残る

- **GIVEN** 印 `0.1.0+<commit A>` で migrate を走らせた後、必ず落ちる試験用の版を並びの末尾に足した D-01
- **WHEN** 印 `0.1.0+<commit B>` の組み立てで退避を有効にして migrate を走らせ（終了コード 1）、そのとき取った退避のファイルを空の別の DB に戻す
- **THEN** 戻した DB の更新の履歴の最後の行は、前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・移行の結果 `attempted` で、その後に結果の行は無い
- **AND** 戻した DB の版の記録に、落ちた版は無い

#### Scenario: 知らない版の当たった DB で止まった migrate は、何も書かない

- **GIVEN** 版の記録に自分の知らない版の行が 1 つある D-01
- **WHEN** 別の組み立ての印で migrate を走らせる
- **THEN** 終了コードは 3 で、更新の履歴の行は走らせる前と同じ

#### Scenario: 版の検査で断られた起動は、何も書かない

- **GIVEN** 版の記録に自分の知らない版の行が 1 つある D-01
- **WHEN** 別の組み立ての印の S-01 を起動する
- **THEN** 終了コードは 3 で、更新の履歴の行は起動する前と同じ

#### Scenario: 更新の履歴の初めの行は、前の版を空にする

- **GIVEN** 更新の履歴の表が空の D-01
- **WHEN** 印 `0.1.0+<commit A>` の組み立てで migrate を走らせる
- **THEN** 更新の履歴の行は 2 行で、どちらも前の版は空・今の版は `0.1.0+<commit A>`、移行の結果は書いた順に `attempted`・`applied`

#### Scenario: 更新の履歴の表が無い D-01 で落ちても、当てる処理は落ちた版の失敗を返し、書けなかったことを警告する

- **GIVEN** 更新の履歴の表を作る版より前の版で必ず落ちる、試験用の並び
- **WHEN** その並びを空の DB に当てる
- **THEN** 当てる処理は落ちた版の失敗を返し、種別 `update_history_unwritten` の警告が出て、DB に更新の履歴の表は無い

#### Scenario: 付与で落ちた migrate は、落ちた版の名前を持たない failed の行を残す

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置き、付与が必ず落ちるようにした D-01
- **WHEN** 印 `0.1.0+<commit B>` を渡して当てる処理を走らせる
- **THEN** 当てる処理は失敗を返し、更新の履歴の最後の行は移行の結果 `failed`・落ちた版の名前は空・今の版 `0.1.0+<commit B>`

#### Scenario: 同じ版の変化を 2 つ同時に書いても、行は 1 つ

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+<commit B>` の起動の書き込みを 2 つ同時に走らせる
- **THEN** 今の版 `0.1.0+<commit B>` の行は 1 行だけ

#### Scenario: commit の取れない組み立ての印も、そのまま今の版に書く

- **GIVEN** 別の印 `0.1.0+<commit A>` の通った行を置いた D-01
- **WHEN** 印 `0.1.0+unknown.debug` を渡して起動の書き込みを走らせる
- **THEN** 更新の履歴の最後の行の今の版は `0.1.0+unknown.debug`

### Requirement: 更新の履歴は、決めた置き場と列の形で持つ

THE SYSTEM SHALL 更新の履歴を次の置き場と列の形で持ち、版をまたいで変えない。形を変えるときは列を足すか、決めた値の一覧に値を足すだけにし、今の列の名前・型・NULL の可否と今の値の意味を変えない。

| 置き場 | 列（名前・型・NULL の可否） |
|---|---|
| schema `core`、表 `update_history`（1 回の版の変化につき 1 行） | `seq` bigint NOT NULL（主キー。書いた順）/ `at` timestamp with time zone NOT NULL / `component` text NOT NULL / `via` text NOT NULL / `from_version` text NULL / `to_version` text NOT NULL / `outcome` text NOT NULL / `applied_versions` text[] NOT NULL / `failed_version` text NULL / `sqlstate` text NULL |

THE SYSTEM SHALL `component` を `S-01`、`via` を `migrate` か `startup`、`outcome` を `attempted`・`applied`・`nothing_to_apply`・`failed`・`backup_failed` のどれかに限る。
THE SYSTEM SHALL 「書いた順」を `seq` で決め、時刻で決めない（S-01 の時計は戻りうる）。

導出元: FR-121。扉 #33。premises の C6（design D4（仮））。review/deep.md R11。
凍結する行の形は後から書き直せないので、ST48 の版の記録と同じく Requirement の表で固定する。

#### Scenario: 更新の履歴の置き場と列の形は、決めた形のまま変わらない

- **WHEN** 空の DB に全部の版を当てる
- **THEN** 更新の履歴の schema・表の名前と、各列の名前・型・NULL の可否はこの Requirement の表と一字一句同じ

#### Scenario: 決めていない値の行は入らない

- **WHEN** 所有者の接続で、`via` が `upgrade` の行を更新の履歴に足す
- **THEN** 足すことは制約に反するとして断られる

#### Scenario: 時計が戻っても、前の版は書いた順で決まる

- **GIVEN** 印 `0.1.0+<commit A>` の行の後に、それより前の時刻を持つ印 `0.1.0+<commit B>` の通った行を書いた D-01
- **WHEN** 印 `0.1.0+<commit C>` の組み立てで migrate を走らせる
- **THEN** 更新の履歴の最後の行の前の版は `0.1.0+<commit B>`

### Requirement: 更新の履歴は書き換えられず、S-01 の実行時の役割は足すことだけができる

THE SYSTEM SHALL 更新の履歴の行を、どの役割からも書き換えさせず、消させない（所有者でも、書き換えの門で断る）。
THE SYSTEM SHALL S-01 の実行時の役割に、更新の履歴を読むことと足すことだけを許す。
THE SYSTEM SHALL 落ちた回の行を、落ちた版の取引を戻した後の別の取引で書く（落ちた版と一緒に戻らない）。
THE SYSTEM SHALL 試みの行を、版を当てる取引とは別の取引で、退避を取る前に commit する（当てた版と一緒に戻らず、退避に入る）。

導出元: FR-121。扉 #33（追記のみ）。深掘り（ST51）Q6。premises の C5（design D3（仮）・D10・D16（仮））。
ST48 の版の記録（実行時の役割は読むだけ）と向きが違うのは、起動の行を実行時の役割が書くから（C2）。

#### Scenario: 更新の履歴の行は書き換えられない

- **GIVEN** 更新の履歴に 1 行ある D-01
- **WHEN** 所有者の接続と S-01 の実行時の役割の接続のそれぞれで、その行の今の版を書き換え、その行を消す
- **THEN** どれも断られ、行は書き換える前と同じ

#### Scenario: S-01 の実行時の役割は、更新の履歴を読み、足せる

- **WHEN** S-01 の実行時の役割で、更新の履歴を読み、行を 1 つ足す
- **THEN** どちらも通る

#### Scenario: 落ちた回の行は、落ちた版と一緒に戻らない

- **GIVEN** 印 `0.1.0+<commit A>` で migrate を走らせた後、必ず落ちる試験用の版を並びの末尾に足した D-01
- **WHEN** 印 `0.1.0+<commit B>` の組み立てで migrate を走らせる
- **THEN** 落ちた版は版の記録に無く、更新の履歴には移行の結果 `failed` の行がある
