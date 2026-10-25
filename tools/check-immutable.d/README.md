# tools/check-immutable.d/

Story ごとの `tools/check-immutable.sh` の節。`tools/check-immutable.sh` が戻しの手順の前に、名前の順に、同じ shell の中で `.` で読む。DB は本体の節を全部走らせた状態で、版は全部当たっている。節の前（読む口の見出しの直前）で、アプリの役割（`ashiato_app`）を作り（`tools/db-roles.sh`）、付与（`crates/server/src/grants.sql`）を当て、`app_psql`（アプリの役割の接続）を定義してある —— 節はアプリの役割から撃ってよい（CH01 design D11）。

- 名前は `YYYYMMDDHHMM_st<NN>-<slug>.sh`（作成時刻。移行と同じく連番にしない —— 並走する Story が番号を取り合う）
- 使える変数・関数は `tools/check-immutable.sh` の読む口の注記のとおり。節の後ろに本体の検査（戻しの並び・読み出しの記録・`app-role`）が続くので、本体の変数（`fail` を NG で 1 にする以外。`MIGS` `ST03_UP` `down_fail` `AU` など）と関数（`psql` `app_psql`）を上書きしない。節の変数は `ST<NN>_`、関数は `st<NN>_` を頭に付ける。利用者の id と本人が書いた行は節が自分で置く。節の中で `exit` しない（後ろの節が走らない。落とすときは `exit 1` だけ）
- `tools/check-immutable.sh` の本体には書き足さない（並走する Story がみな末尾を取り合い、どの 2 本を合わせても衝突した。2026-10-15）。
  `feat/st*` が本体の末尾（最後の節の後ろ）に書き足すと `tools/check-shared-appends.sh` が落とす（途中の既存の節の中身を直すのは通す。**本体の途中に新しい節の見出し**（`# ==== ST<NN>` / `echo "== ST<NN>:` など）を足しても落とす。読む口のかたまりをそのまま動かすだけは通す）
- **節は本体の各節の戻し手順（`.down.sql` を当てて、その版をもう一度当てる）の後に読まれ、古い門の上で走る。** 当て直した版より後の版が置き換えた関数（門・錠）は、当て直した版の本体に戻っている。
  新しい門・錠を前提にする節は、`psql -c "DROP SCHEMA core CASCADE;"` の後に `"${MIGS[@]}"` を全部当てて schema を作り直してから撃つ（`202610231200_st41-event-end-time.sh` の頭の形）。
  **作り直すと付与（`grants.sql`）も消える**ので、続けて `./tools/db-roles.sh` と `grants.sql`（`ashiato_owner` で）を当て直す —— 当て直さないと、後ろの節の `app_psql` が門・錠でなく権限で断られ、拒否を見る検査が別の理由で OK になる（review/code.md R17）
