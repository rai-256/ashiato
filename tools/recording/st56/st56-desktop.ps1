# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
# 録画専用・前半。ST56 の PC の `setup` を Windows Sandbox で実際に通す。
# Sandbox の網は切ってあるため、loopback の送り先への接続確認は「保存しました。繋がったら送ります」になる。
# それでも設定は保存され、資格情報は credential.bin にのみ置かれる。その後に収集を起こして
# フォルダ「ST56 の設定」を前景にし、未送信に記録が積まれるところまで撮る。
$ErrorActionPreference = 'Stop'
$work = $env:REC_WORK
$state = $env:ASHIATO_STATE_DIR
$exe = Join-Path $env:REC_PAYLOAD 'ashiato-collector-windows.exe'

function Front([string]$Title) {
  $sh = New-Object -ComObject WScript.Shell
  for ($i = 0; $i -lt 20; $i++) { if ($sh.AppActivate($Title)) { return }; Start-Sleep -Milliseconds 500 }
  throw "「$Title」の窓を前景にできない"
}

New-Item -ItemType Directory -Path $state -Force | Out-Null
# 共有契約の valid vector をそのまま貼る。Sandbox の網は切ってあるので接続確認は届かないが、
# 保存と秘密の置き方は本物の `setup` と同じ経路で確かめられる。
$setup = 'ashiato-collector:1:eyJ1cmwiOiJodHRwczovL3BjLmV4YW1wbGUudHMubmV0L2FwaSIsInRva2VuIjoidG9rZW4tZm9yLXRlc3QtMDEyMzQ1Njc4OSJ9'

Show-RecCaption '1. PC の setup に設定文字列を貼る（資格情報は画面に出さない）'
$start = New-Object System.Diagnostics.ProcessStartInfo
$start.FileName = $exe
$start.Arguments = 'setup'
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$process = New-Object System.Diagnostics.Process
$process.StartInfo = $start
[void]$process.Start()
$inputBytes = [Text.Encoding]::ASCII.GetBytes($setup + "`n")
$process.StandardInput.BaseStream.Write($inputBytes, 0, $inputBytes.Length)
$process.StandardInput.Close()
$setupOutput = $process.StandardOutput.ReadToEnd() + $process.StandardError.ReadToEnd()
$process.WaitForExit()
if ($process.ExitCode -ne 0) { throw "setup が失敗した（rc=$($process.ExitCode)）: $setupOutput" }
if ($setupOutput -notmatch '保存しました。繋がったら送ります') { throw "届かない送り先の結果が違う: $setupOutput" }
Start-Sleep -Seconds 4

Show-RecCaption '2. 設定は保存され、資格情報の平文は置き場の設定ファイルに無い'
$settings = Join-Path $state 'settings.json'
$credential = Join-Path $state 'credential.bin'
if (-not (Test-Path $settings) -or -not (Test-Path $credential)) { throw 'settings.json または credential.bin が無い' }
$textFiles = Get-ChildItem -Path $state -Recurse -File | Where-Object { $_.Extension -ne '.bin' }
foreach ($file in $textFiles) {
  if ((Get-Content $file.FullName -Raw -Encoding UTF8) -match 'token-for-test-0123456789') {
    throw "資格情報の平文が $($file.FullName) に残っている"
  }
}
Start-Sleep -Seconds 4

Show-RecCaption '3. 設定済みの収集を起こし、フォルダ「ST56 の設定」を前景にする'
$collector = Start-Process $exe -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 4
if ($collector.HasExited) { throw "収集アプリがすぐ終わった（rc=$($collector.ExitCode)）" }
$memo = Join-Path $work 'ST56 の設定'
New-Item -ItemType Directory -Path $memo -Force | Out-Null
[System.IO.File]::WriteAllText((Join-Path $memo '接続を設定しました.txt'), '設定後も収集を続ける録画用の記録。', (New-Object System.Text.UTF8Encoding($true)))
Start-Process explorer.exe -ArgumentList "`"$memo`""
Front 'ST56 の設定'
Start-Sleep -Seconds 6

Show-RecCaption '4. 設定後の PC の記録が未送信に積まれた。この後で録画用サーバへ送る'
$outbox = Join-Path $state 'outbox.jsonl'
$text = ''
for ($i = 0; $i -lt 20; $i++) {
  if (Test-Path $outbox) { $text = Get-Content $outbox -Raw -Encoding UTF8 }
  if ($text -like '*ST56 の設定*') { break }
  Start-Sleep -Milliseconds 500
}
if ($text -notlike '*ST56 の設定*') { throw "未送信にフォルダの題名が無い（$outbox）" }
Start-Sleep -Seconds 4
Stop-Process -Id $collector.Id -Force -ErrorAction SilentlyContinue
Show-RecCaption ''
