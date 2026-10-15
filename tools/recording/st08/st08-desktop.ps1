# SPDX-License-Identifier: AGPL-3.0-only
# 録画専用・前半（ハーネスの record-run が Windows Sandbox の中で走らせる。デスクトップ全体が撮られている）。
# ST08「PC のブラウザ履歴を集める」: ブラウザでページ A → ページ B、ページ C を新しいタブで開いてすぐ閉じる →
# 収集アプリを起こす → 初回の取得で、訪問 1 件ごとの記録が積まれる（一瞬だけ開いたページも入る）。
# 「消えた訪問」の記録は次の取得（24 時間後）でしか起きないので撮らない（単体・結合のテストが持つ）。
# Sandbox の網は切ってあるので記録は送れずに積まれ、閉じた後に tools/record-env.sh after-windows がサーバへ送る。
#
# 失敗は throw（ハーネスの runner.ps1 が result.json に書く）。工程の見出しは Show-RecCaption（下端の帯。前景は取らない）。
$ErrorActionPreference = 'Stop'
$work = $env:REC_WORK
$state = $env:ASHIATO_STATE_DIR
$hold = 5
$url = { param($n) 'file:///' + ($work -replace '\\', '/') + "/$n.html" }

$edge = 'HKLM:\SOFTWARE\Policies\Microsoft\Edge'
New-Item -Path $edge -Force | Out-Null
Set-ItemProperty -Path $edge -Name HideFirstRunExperience -Value 1 -Type DWord
Set-ItemProperty -Path $edge -Name DefaultBrowserSettingEnabled -Value 0 -Type DWord
foreach ($p in @(@('page-a', 'ページ A'), @('page-b', 'ページ B'), @('page-c', 'ページ C'))) {
  $html = "<!doctype html><meta charset=utf-8><title>$($p[1]) — 録画</title>" +
          "<body style='font:64px sans-serif;margin:80px'>$($p[1])<p style='font-size:28px'>ST08 の録画用のページ</p>"
  [System.IO.File]::WriteAllText((Join-Path $work "$($p[0]).html"), $html, (New-Object System.Text.UTF8Encoding($false)))
}

Show-RecCaption '1. ブラウザでページ A を開く（収集アプリはまだ起こしていない）'
Start-Process msedge.exe -ArgumentList @('--no-first-run', '--no-default-browser-check', (& $url 'page-a'))
Start-Sleep -Seconds $hold
Show-RecCaption '2. ページ B へ移る'
Start-Process msedge.exe -ArgumentList @((& $url 'page-b'))
Start-Sleep -Seconds $hold
Show-RecCaption '3. ページ C を新しいタブで開き、すぐ閉じる（一瞬だけ見たページも、履歴には 1 件として残る）'
$sh = New-Object -ComObject WScript.Shell
Start-Process msedge.exe -ArgumentList @((& $url 'page-c'))
$front = $false
for ($i = 0; $i -lt 20 -and -not $front; $i++) { $front = $sh.AppActivate('ページ C'); if (-not $front) { Start-Sleep -Milliseconds 500 } }
if (-not $front) { throw '「ページ C」の窓を前景にできない（Ctrl+W を別の窓へ送らない）' }
Start-Sleep -Seconds 1
$sh.SendKeys('^w')
Start-Sleep -Seconds $hold

Show-RecCaption '4. ブラウザを閉じる（履歴をディスクへ書かせる）'
Get-Process msedge -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object { $_.CloseMainWindow() | Out-Null }
for ($i = 0; $i -lt 30 -and (Get-Process msedge -ErrorAction SilentlyContinue); $i++) { Start-Sleep -Milliseconds 500 }
Get-Process msedge -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
$hist = Join-Path $env:LOCALAPPDATA 'Microsoft\Edge\User Data\Default\History'
if (-not (Test-Path $hist)) { throw "Edge の履歴の DB が無い（$hist）" }

Show-RecCaption '5. PC の収集アプリを起こす（初回の取得で、全プロファイルの履歴を読む）'
$exe = Join-Path $env:REC_PAYLOAD 'ashiato-collector-windows.exe'
$collector = Start-Process $exe -PassThru -WindowStyle Hidden
$outbox = Join-Path $state 'outbox.jsonl'
$want = @('page-a.html', 'page-b.html', 'page-c.html')
$text = ''
for ($i = 0; $i -lt 120; $i++) {
  if ($collector.HasExited) { throw "収集アプリが途中で終わった（rc=$($collector.ExitCode)。$state\collector.log）" }
  if (Test-Path $outbox) { $text = Get-Content $outbox -Raw -Encoding UTF8 }
  if ($text -like '*c02-browser-history*' -and -not ($want | Where-Object { $text -notlike "*$_*" })) { break }
  Start-Sleep -Milliseconds 500
}
if ($text -notlike '*c02-browser-history*') { throw "60 秒たっても履歴の記録が積まれない（$outbox）" }
$missing = $want | Where-Object { $text -notlike "*$_*" }
if ($missing) { throw "履歴の記録に無い訪問: $($missing -join ' / ')（$outbox）" }
$n = ($text -split "`n" | Where-Object { $_ -like '*c02-browser-history*' }).Count
Show-RecCaption "6. 訪問 $n 件が記録として積まれた（一瞬だけ開いたページ C も）。この後、サーバへ送って 1 日の画面で見る"
Start-Sleep -Seconds 4
Stop-Process -Id $collector.Id -Force -ErrorAction SilentlyContinue
Show-RecCaption ''
