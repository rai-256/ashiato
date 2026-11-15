# impact-review — st58-update-apply-rollback

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 medium, IMP-05 medium, IMP-06 low, IMP-07 medium (reflected)

## R1 — 更新の履歴の「前の版」を読む処理が rolled_back を数えず、戻した後の次の更新の from_version を誤りうる（依存の見落とし）
- 対象: impact.dependencies（T3 の読む側が無い）/ impact.targets[T3]
- 種類: 依存の見落とし
- 指摘: crates/server/src/update_history.rs:127-136 の `previous_version` は `WHERE outcome IN ('applied','nothing_to_apply') ORDER BY seq DESC LIMIT 1` で「今の版」を決め、次の更新の `from_version` に使う。design D26（design.md:190）は、戻した回に `applied`/`nothing_to_apply` を「当てた新しい版の名前」で書き直してから `rolled_back` を足す。`rolled_back` はこの IN に入らないので、戻した後の最後の通った行は落ちた新しい版を指し、次に通る更新の「前の版」が実際に動いていた版と食い違う。D5（T3）の dependencies は「前の版の DB が rolled_back を拒む」「書き出しを読む側」だけで、サーバ自身のこの読みを挙げていない。tasks/spec にもこの読みの直しは見当たらない（grep で previous_version / from_version の記述なし）。版の記録は凍結した表で後から直せない（memory の one_way_door と同じ性質）。
- 直し方の案: dependencies に `previous_version` を読む側として足し、rolled_back の行の to_version を「今の版」とみなす（または IN に足して seq で並べる）かを design で決め、Scenario に戻した後の更新の from_version を足す。
- 処置: fixed: dependencies に D13（update_history.rs の previous_version）と E14 を足し、IMP-07（medium）として findings・scope.residual・acceptance・overview に出した。直し方の案は IMP-07 の options

## R2 — IMP-04 の「止まる」は unknowns と食い違う言い切り（言い切り）
- 対象: impact.overview / impact.scenarios[S1].result / impact.findings[IMP-04].unknowns
- 種類: 言い切り
- 指摘: overview は「直した版が出るまでサーバは止まる」、S1 は「0.5.0 のまま落ち続ける」と断定するが、IMP-04 の unknowns は「新しい移行の無い版なら前の版のインストーラで走らせ直すと前の版で起きうるか（…確かめていない）」と書く。design.md:157 も、段が `listening` なら「どちらの版のインストーラも 8 の残りを進めて終える」と読め、前の版のインストーラを走らせ直す道が実際に何をするかは未確認。止まる・戻れないを確かめずに断定している一方、IMP-04 の options 2 番目（プログラムだけ戻す口）の要否もここで決まる。
- 直し方の案: overview・S1 に「走らせ直しで前の版が起きるかは未確認」と限定を足すか、Windows の通しで確かめてから断定する。
- 処置: fixed: overview と S1 の言い切りを外し、前の版のインストーラで走らせ直したときは確かめていないと書いた。IMP-04 の unknowns に印が残っている場合を足した

## R3 — D9（PC の同時起動）の basis と evidence が、IMP-05 の guess と不整合（根拠）
- 対象: impact.dependencies[D9] / impact.findings[IMP-05]
- 種類: 根拠
- 指摘: D9 は `holds: no`・`basis: observed` だが、E10（autostart.rs:114 の `current_exe`）が示すのは「自動起動の台本は動いている実行ファイルを指す」ことだけで、「入れ替え後も本人が置いた場所の実行ファイルが前の版のまま残る」は design D12（仮）の設計の予定にすぎない（まだコードに無い）。E11 も premises の記述。実物で観測できた事実ではないので、IMP-05 が `guess` なのに D9 を `observed`・`holds: no` と置くのは強すぎる。
- 直し方の案: D9 を `basis: derived` か `holds: unknown` にそろえる。
- 処置: fixed: D9 を basis derived・refs D12 にし、assumes に「design D12（仮）の予定」と書いた

## R4 — mixed の can「前の版の収集アプリの未送信は新しい版のサーバに届く」を支える ref が無い（根拠）
- 対象: impact.situations[mixed].can
- 種類: 根拠
- 指摘: refs はインストーラの Requirement と D1 だけで、サーバが前の版の収集アプリの送信契約を受け付けることを示す Scenario/実物が挙がっていない。spec の近い Scenario は PC の入れ替え前後の未送信（distribution/spec.md:128, 134）で、サーバの更新後に旧版の収集アプリの送信が届く場面を扱っていない。仕様の写しではなく、確かめていない「できる」。
- 直し方の案: 該当 Scenario を refs に足すか、can から外して未確認と書く。
- 処置: fixed: situations[mixed].can を確かめていないこととして書き直し、basis を unconfirmed にした

## R5 — R2・R7・R10・R11・R12・R13 の legit「見つけていない」の探し方が示されていない（言い切り）
- 対象: impact.rules[R2,R7,R10,R11,R12,R13].legit
- 種類: 言い切り
- 指摘: R2 の「正当な利用を止める場面を見つけていない」は限定付きで書かれているが、R2 は「この回の取り終えた退避が無い回」を戻さない規則で、MIGRATE_BACKUP=off（migrate_backup.rs:28）がシステムの環境に付いたインストールや退避が取れなかった回の扱いは確かめた形跡が無い（design D26 は backup_failed を移行が DB に触れない前提で外している）。確かめた範囲を書かないと「見つけていない」が検索の結果か、確かめなかっただけか分からない。
- 直し方の案: R2 の legit に、確かめた場面（退避が無効・退避の失敗・終了コード 2〜4）を書く。
- 処置: fixed: R2 の legit に確かめた場面（終了コード 2・3・4・退避の前に落ちた回・退避に失敗した回）と確かめていない場面（退避を切ったインストール）を書いた。R7・R10〜R13 は当てはめた範囲（Scenario と利用場面の 6 つ）を添えた

## R6 — IMP-02 の消した記録が残る期間は、既にある毎日のバックアップとの差が書かれていない（重大度）
- 対象: impact.findings[IMP-02] / impact.memory[0]
- 種類: 重大度
- 指摘: memory[0] の `why` は「消したつもりの記録が同じ PC に残る期間がある」と書くが、毎日のバックアップ（30 日分。memory[2] 自身が言及）も削除を巻き戻さない写しである。IMP-02 は追加で増える分（落ちた DB と退避の数か月）に絞れており、medium は妥当だが、unknowns の「落ちた DB を書き出しやバックアップが読むか」は書き出し（export.rs の対象表）を見れば機械的に確かめられる。確かめられることを unknown のまま残している。
- 直し方の案: export.rs とバックアップの対象を見て unknowns を閉じるか、「確かめていない」と明記する。
- 処置: fixed: 書き出しはサーバが繋ぐ記録の DB だけを読むことを IMP-02 に書き、unknowns から外した。確かめる途中で、毎日のバックアップ（ST30）がまだ作られていないと分かった。仕様の「30 日分は別に残る」との食い違いとして open に出し、keep.1・memory・R4・S3 を直した

## R7 — scope.preconditions の「戻した」の値の使い道が、Q9 の後の仕様と食い違う（言い切り）
- 対象: impact.scope.preconditions[「戻した」の値は…]
- 種類: 言い切り
- 指摘: 「『戻した』の値は、移行が落ちて退避から DB を戻した回の行だけに使う」と限っているが、Q9 の後は `rolled_back` を、移行は通ったが待ち受けを開かずに戻した回にも使う（specs/distribution/spec.md:504-514 の Scenario、:881 の「移行は通ったが新しい版が待ち受けを開かずに戻した回は当てた版に…名前を持つ」、:927）。同じ go.json の R16・residual 4 つ目は 2 つの回を書いており、scope の前提だけが第 1 回の形のまま残っている。範囲の前提としては、後の Story が「rolled_back なら移行が落ちた回」と読む根拠になる。
- 直し方の案: 「移行が落ちた回と、待ち受けを開かずに戻した回の行に使う。後者は結果の行を書き直さず、当てた版を持つ」に書き直す。
- 処置: fixed: scope.preconditions の「戻した」の値の行を、移行が落ちた回と待ち受けを開かずに戻した回の 2 つの置き方（後者は当てた版の名前を持つ）に書き直した

## R8 — 一度出ると書き直せない「failed の有無で 2 つの回を読み分ける」約束が、memory と D7 に載っていない（依存の見落とし）
- 対象: impact.memory[3]（one_way_door）/ impact.dependencies[D7]
- 種類: 依存の見落とし
- 指摘: Q9 で `rolled_back` の行は 2 つの形を持つ（直前に `failed` があれば移行が落ちた回、無ければ待ち受けを開かずに戻した回。後者は `applied_versions` に当てた版を入れる。spec.md:484, :886, :927）。spec 自身が「一度出た行は書き直せない」と書く。memory[3] の why は「値と向き」だけで、この読み分けの約束と、`applied_versions` を `rolled_back` では別の意味（戻した回に当てた版）で使うことを挙げていない。D7（書き出しを読む側）の affects も「戻した回を読み違えうる」までで、読み違える具体（`failed` が無い `rolled_back` を「記録の不整合」や「通った回」と読む）が無い。residual 4 つ目にだけ出ており、扉と依存の表からは見えない。data-durability/spec.md:8,:38 が説明書の語を固定しているので手当ては実在するが、それは書き出しの説明書を読む側に限る。
- 直し方の案: memory[3] の text/why に読み分けと `applied_versions` の使い方を足し、D7 の affects に「`failed` が無い `rolled_back`」を書く。
- 処置: fixed: memory[3]（one_way_door）に 2 つの形と failed の有無での読み分けを足し、refs に Q9・D26 を足した。D7 の affects と verify に「failed の無い rolled_back」の読み違えを足した

## R9 — Q8 で残る「位置が開くまで取れない」は取り戻せない欠けで、R9 の blocks_legit=false と findings に載っていない（重大度）
- 対象: impact.rules[R9] / impact.findings（該当 IMP 無し）
- 種類: 重大度
- 指摘: R9 の legit は「位置が欠ける」と書きつつ blocks_legit を false にし、IMP を立てていない。spec.md:54-59 と design D10 の表では、裏の位置の権限を断った端末は更新のたびに開くまで位置の記録が取れず、spec 自身が「取らなかった期間の記録は後から作れない」と書く。通知の権限が無い端末（spec.md:55）は通知も出ず、位置の権限が無い端末は開いても位置は戻らない（design D10 の表の最終行）。生存信号に「権限」と出るが、本人が見に行かなければ気づけない。「あるとよい」権限を断った正当な利用者が、更新のたびに位置を失う。高 (high) ほどではない（位置以外は取り続け、生存信号で外から見える）が、今は findings にも residual にも無く、R9 の legit にだけ書かれて blocks が false。さらに legit の「開くまで位置が欠ける」は位置の権限なしの端末には当たらない（開いても戻らない）。
- 直し方の案: IMP（medium か low）として足し、通知が出ない端末の扱い（開くまで気づけない）と取り戻せない欠けを unknowns・residual に書き、R9 の blocks_legit を true か "unknown" にする。
- 処置: kept: blocks_legit は false のまま。位置が開くまで欠けることは、Q8 で本人がその帰結（「位置は開くまで止まる」）を見て選んだ側で、前（ST58 より前）は開くまで全部が止まっていたので、この規則が新しく止める正当な利用ではない。R9 の legit を直し、後から取れないこと・通知の権限が無い端末は稼働状況からしか気づけないこと・位置の権限が無い端末は今と同じく位置を取らないことを書いた

## R10 — 受け手から起こした後に OS が立て直すと、位置を除いた種別の前提が崩れうるが D10 の依存に無い（新しい帰結）
- 対象: impact.dependencies[D10] / impact.scenarios
- 種類: 新しい帰結
- 指摘: design D10 は、印のファイルを `onCreate` が読んで消し、受け手から起こしたサービスの目印はプロセスが落ちれば消えるとし、「開く前に OS が立て直したときは今の立て直しと同じ形になる（変えない）」と書く。実物の `foregroundServiceType()`（collector-android/.../LocationService.kt:877-882）は `ACCESS_FINE_LOCATION` だけを見て、あれば `location` を返す。よって裏の位置の権限が無い端末で、受け手から `specialUse` で起こした後にプロセスが落ち `START_STICKY` で立て直されると、印は消えているので `location` で `startForeground` を呼ぶ。裏から位置の型を立てられないなら（premises P7 は、許されていない端末は位置を除いた種別でだけ始められるとする）、LocationService.kt:861-867 のコメントが言う SecurityException で、アプリ利用の収集まで止まる。D10 の assumes・affects・verify（adb install -r の直後だけ）にも acceptance にもこの場面が無い。確かさは guess（OS の立て直しの時機と、立て直しが裏の位置の制限に当たるかは実物で確かめていない）。
- 直し方の案: D10 に「受け手から起こした後の立て直し」を依存として足し（holds: unknown）、計測テストにプロセスを落として立て直す場面を足す。または立て直しの種別も `specialUse` に倒す設計を決める。
- 処置: fixed: IMP-08（medium・guess）として findings に足し、D10 の affects と leads_to、scope.residual、acceptance、overview に出した。根拠に E15（LocationService.kt:878）を足した。仕様は reading の段では直さない（直すかは合意で本人が決める）

## R11 — IMP-07 の reflected は仕様を正しく指すが、options が反対向きの案のまま残り、options が反対向きの案のまま残る（閉じ方）
- 対象: impact.findings[IMP-07].options / unknowns / resolution
- 種類: 閉じ方
- 指摘: reflected_in の Requirement と 3 つの Scenario は実在し、spec.md:480・:516-520 が resolution のとおり「前の版の読みを変えず、`rolled_back` を数えない」を固定している。コードも `previous_version`（update_history.rs:127-136）が `applied`/`nothing_to_apply` だけを読む。一方で options の 1 つ目は「前の版の読みに rolled_back を足す」と、採用した形と逆の案が残っている。unknowns の「控えから写す回で `applied`/`nothing_to_apply` を書かないことは確かめる前」は、spec.md:522-526 の Scenario（写した後に `applied` と `nothing_to_apply` が足されない）が固定しているので、unknowns を閉じる材料はあるが、overview の「起きない形に直した」は試験が通るまでは設計の上の話。
- 直し方の案: options を採用した案と捨てた案の理由に書き直し、unknowns を spec.md:522 の Scenario に結びつけて閉じる（または試験が通るまで未確認と書く）。
- 処置: fixed: IMP-07 の options を採った形（Q9）と採らなかった形に書き直し、unknowns を控えの経路の Scenario に結び、試験は下流で書くと書いた。resolution と overview の「起きない形に直した」に「仕様の上では」「試験は下流で書く」を添えた
