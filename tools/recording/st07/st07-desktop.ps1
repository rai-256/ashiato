# SPDX-License-Identifier: AGPL-3.0-only
# 録画専用・前半（ハーネスの record-run が Windows Sandbox の中で走らせる。デスクトップ全体が撮られている）。
# ST07「PC のアクティブウィンドウを集める」: 収集アプリを起こす → フォルダ「メモ A」→ ブラウザでページ A → ページ B →
# フォルダへ戻る（Sandbox（24H2）にはメモ帳が無いので、どの Windows にもあるエクスプローラーの窓を使う。2026-10-15 実測）。
# 切り替えるたびに 1 件ずつ記録が積まれる（FR-12）ことを、収集アプリの未送信の置き場で確かめる。
# Sandbox の網は切ってあるので記録は送れずに積まれ、閉じた後に tools/record-env.sh after-windows がサーバへ送る。
# 後半（st07-result.rec.ts）が、届いた記録を 1 日の画面で見せる。
#
# 失敗は throw（ハーネスの runner.ps1 が result.json に書く）。工程の見出しは Show-RecCaption（下端の帯。前景は取らない）。
$ErrorActionPreference = 'Stop'
$work = $env:REC_WORK
$state = $env:ASHIATO_STATE_DIR
$hold = 6   # 1 つの窓を前景に置いておく秒数（見るための停止。記録は切り替えた瞬間に積まれる）

function Front([string]$Title) {
  # 題名の一部で窓を前景へ。AppActivate は前景にできなければ false を返す
  $sh = New-Object -ComObject WScript.Shell
  for ($i = 0; $i -lt 20; $i++) { if ($sh.AppActivate($Title)) { return }; Start-Sleep -Milliseconds 500 }
  throw "「$Title」の窓を前景にできない"
}

# 用意: ページ 2 枚とフォルダ。ブラウザの初回の案内は出さない（Sandbox は毎回まっさらなので、出ると画面が案内で埋まる）
$edge = 'HKLM:\SOFTWARE\Policies\Microsoft\Edge'
New-Item -Path $edge -Force | Out-Null
Set-ItemProperty -Path $edge -Name HideFirstRunExperience -Value 1 -Type DWord
Set-ItemProperty -Path $edge -Name DefaultBrowserSettingEnabled -Value 0 -Type DWord
foreach ($p in @(@('page-a', 'ページ A'), @('page-b', 'ページ B'))) {
  $html = "<!doctype html><meta charset=utf-8><title>$($p[1]) — 録画</title>" +
          "<body style='font:64px sans-serif;margin:80px'>$($p[1])<p style='font-size:28px'>ST07 の録画用のページ</p>"
  [System.IO.File]::WriteAllText((Join-Path $work "$($p[0]).html"), $html, (New-Object System.Text.UTF8Encoding($false)))
}
$memo = Join-Path $work 'メモ A'
New-Item -ItemType Directory -Path $memo -Force | Out-Null
[System.IO.File]::WriteAllText((Join-Path $memo 'ST07 の録画.txt'), "この窓を前景に置いている間は、エクスプローラーの記録が 1 件。", (New-Object System.Text.UTF8Encoding($true)))

Show-RecCaption '1. PC の収集アプリを起こす（画面は無い。裏で前景の窓を読む）'
$exe = Join-Path $env:REC_PAYLOAD 'ashiato-collector-windows.exe'
$collector = Start-Process $exe -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 4
if ($collector.HasExited) { throw "収集アプリがすぐ終わった（rc=$($collector.ExitCode)。$state\collector.log）" }

Show-RecCaption '2. エクスプローラーでフォルダ「メモ A」を開く'
Start-Process explorer.exe -ArgumentList "`"$memo`""
Front 'メモ A'
Start-Sleep -Seconds $hold

Show-RecCaption '3. ブラウザでページ A を開く'
Start-Process msedge.exe -ArgumentList @('--no-first-run', '--no-default-browser-check', ('file:///' + ($work -replace '\\', '/') + '/page-a.html'))
Front 'ページ A'
Start-Sleep -Seconds $hold

Show-RecCaption '4. 同じブラウザでページ B へ（URL と題名が変わる）'
Start-Process msedge.exe -ArgumentList @(('file:///' + ($work -replace '\\', '/') + '/page-b.html'))
Front 'ページ B'
Start-Sleep -Seconds $hold

Show-RecCaption '5. フォルダ「メモ A」へ戻る'
Front 'メモ A'
Start-Sleep -Seconds $hold

Show-RecCaption '6. 収集アプリが積んだ記録を確かめる（網を切ってあるので送らずに未送信に積まれる）'
$outbox = Join-Path $state 'outbox.jsonl'
$want = @('メモ A', 'ページ A', 'ページ B')
$text = ''
for ($i = 0; $i -lt 20; $i++) {
  if (Test-Path $outbox) { $text = Get-Content $outbox -Raw -Encoding UTF8 }
  if (-not ($want | Where-Object { $text -notlike "*$_*" })) { break }
  Start-Sleep -Milliseconds 500
}
$missing = $want | Where-Object { $text -notlike "*$_*" }
if ($missing) { throw "未送信に無い題名: $($missing -join ' / ')（$outbox）" }
$n = ($text -split "`n" | Where-Object { $_.Trim() }).Count
Show-RecCaption "7. 記録 $n 件が積まれた。この後、サーバへ送って 1 日の画面で見る"
Start-Sleep -Seconds 4
Stop-Process -Id $collector.Id -Force -ErrorAction SilentlyContinue
Show-RecCaption ''
