# SPDX-License-Identifier: AGPL-3.0-only
# 録画専用・前半（ハーネスの record-run が Windows Sandbox の中で走らせる。デスクトップ全体が撮られている）。
# ST50「送信契約の版」: サーバが ST50 より古い（版を問い合わせる口 `GET /contract` が 404）→ 収集アプリは送らずに PC に残し、
# 「サーバを更新してください」の通知を出す → サーバを更新した（版の答えが読める）→ 起こし直した最初の送る契機で通知が消える。
#
# **古いサーバは Sandbox の中に立てる。** Sandbox の網は切ってあり、録画用のサーバ（ST50 の版を返す）には届かない。
# ここでは loopback に小さな偽のサーバを立て、収集アプリの接続先をそこへ向ける:
#   old … どの口も 404（ST50 より前のサーバ。版を問い合わせる口が無い）
#   new … `GET /contract` は `{"contract_version":1}`、ほかは 503（受け取りは閉じた後に本物のサーバへ。下）
# 503 は一時的な失敗なので、積んだ記録は PC に残ったまま。閉じた後に tools/record-env.sh after-windows が録画用のサーバへ送り、
# 後半（st50-result.rec.ts）が、PC に残していた記録が 1 日の画面に届いたところを見せる。
# 通知の出し入れは収集アプリの置き場（contract-wait.json の notified_at）でも確かめる。
#
# 失敗は throw（ハーネスの runner.ps1 が result.json に書く）。工程の見出しは Show-RecCaption（下端の帯。前景は取らない）。
$ErrorActionPreference = 'Stop'
$work = $env:REC_WORK
$state = $env:ASHIATO_STATE_DIR
$hold = 6
$port = 18790
$mode = Join-Path $work 'server-mode.txt'
$wait = Join-Path $state 'contract-wait.json'
$exe = Join-Path $env:REC_PAYLOAD 'ashiato-collector-windows.exe'

function Set-ServerMode([string]$m) { [System.IO.File]::WriteAllText($mode, $m) }

function Get-NotifiedAt {
  if (-not (Test-Path $wait)) { return $null }
  return (Get-Content $wait -Raw -Encoding UTF8 | ConvertFrom-Json).notified_at
}

function Wait-Until([scriptblock]$Check, [string]$Message) {
  for ($i = 0; $i -lt 60; $i++) { if (& $Check) { return }; Start-Sleep -Milliseconds 500 }
  throw $Message
}

function Start-Collector {
  $p = Start-Process $exe -PassThru -WindowStyle Hidden
  Start-Sleep -Seconds 2
  if ($p.HasExited) { throw "収集アプリがすぐ終わった（rc=$($p.ExitCode)。$state\collector.log）" }
  return $p
}

# 偽のサーバ（TcpListener。HttpListener は URL の予約が要る）。1 回ごとに mode のファイルを読んで答えを決める。
# 本文は読み捨てる（ASCII で読むので、バイト数と文字数が一致する）
Set-ServerMode 'old'
$server = Start-Job -ArgumentList $port, $mode -ScriptBlock {
  param($port, $mode)
  $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $port)
  $listener.Start()
  while ($true) {
    $client = $listener.AcceptTcpClient()
    try {
      $stream = $client.GetStream()
      $reader = New-Object System.IO.StreamReader($stream, [System.Text.Encoding]::ASCII)
      $path = ($reader.ReadLine() -split ' ')[1]
      $len = 0
      while ($line = $reader.ReadLine()) { if ($line -match '^(?i)content-length:\s*(\d+)') { $len = [int]$Matches[1] } }
      if ($len -gt 0) { $null = $reader.ReadBlock((New-Object char[] $len), 0, $len) }
      $m = [System.IO.File]::ReadAllText($mode).Trim()
      if ($m -eq 'new' -and $path -like '/contract*') { $status = '200 OK'; $body = '{"contract_version":1}' }
      elseif ($m -eq 'new') { $status = '503 Service Unavailable'; $body = '' }
      else { $status = '404 Not Found'; $body = '' }
      $bytes = [System.Text.Encoding]::UTF8.GetBytes($body)
      $head = [System.Text.Encoding]::ASCII.GetBytes(
        "HTTP/1.1 $status`r`nContent-Type: application/json`r`nContent-Length: $($bytes.Length)`r`nConnection: close`r`n`r`n")
      $stream.Write($head, 0, $head.Length)
      $stream.Write($bytes, 0, $bytes.Length)
      $stream.Flush()
    } catch {
    } finally {
      $client.Close()
    }
  }
}
Start-Sleep -Seconds 3
if ($server.State -ne 'Running') { throw "偽のサーバが立たない: $(Receive-Job $server 2>&1)" }
$env:ASHIATO_BASE_URL = "http://127.0.0.1:$port"

$memo = Join-Path $work 'メモ ST50'
New-Item -ItemType Directory -Path $memo -Force | Out-Null
[System.IO.File]::WriteAllText((Join-Path $memo 'ST50 の録画.txt'), "この窓を前景に置いている間は、エクスプローラーの記録が 1 件。", (New-Object System.Text.UTF8Encoding($true)))

Show-RecCaption '1. サーバが ST50 より古い（版を問い合わせる口が無い）。PC の収集アプリを起こす'
$collector = Start-Collector
Wait-Until { Get-NotifiedAt } "更新を促す通知を出した印が付かない（$wait）"
Show-RecCaption '2. 最初の送る契機で版が分からない → 送らずに PC に残し、「サーバを更新してください」の通知'
Start-Sleep -Seconds $hold

Show-RecCaption '3. その間も集める: フォルダ「メモ ST50」を開く'
Start-Process explorer.exe -ArgumentList "`"$memo`""
$sh = New-Object -ComObject WScript.Shell
Wait-Until { $sh.AppActivate('メモ ST50') } '「メモ ST50」の窓を前景にできない'
Start-Sleep -Seconds $hold

Show-RecCaption '4. 通知の一覧にも残っている'
Start-Process 'ms-actioncenter:' -ErrorAction SilentlyContinue
Start-Sleep -Seconds $hold
$sh.SendKeys('{ESC}')

$outbox = Join-Path $state 'outbox.jsonl'
Wait-Until { (Test-Path $outbox) -and ((Get-Content $outbox -Raw -Encoding UTF8) -like '*メモ ST50*') } "未送信に「メモ ST50」が無い（$outbox）"
Stop-Process -Id $collector.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

Show-RecCaption '5. サーバを更新した（版の答えが読める）。収集アプリを起こし直す'
Set-ServerMode 'new'
$collector = Start-Collector
Wait-Until { -not (Get-NotifiedAt) } "更新したのに通知を出した印が消えない（$wait）"
Show-RecCaption '6. 最初の送る契機で待つ状態を抜け、通知は消える'
Start-Process 'ms-actioncenter:' -ErrorAction SilentlyContinue
Start-Sleep -Seconds $hold
$sh.SendKeys('{ESC}')

$n = ((Get-Content $outbox -Raw -Encoding UTF8) -split "`n" | Where-Object { $_.Trim() }).Count
Show-RecCaption "7. PC に残していた記録 $n 件。この後、更新したサーバへ送って 1 日の画面で見る"
Start-Sleep -Seconds 4
Stop-Process -Id $collector.Id -Force -ErrorAction SilentlyContinue
Stop-Job $server -ErrorAction SilentlyContinue
Remove-Job $server -Force -ErrorAction SilentlyContinue
Show-RecCaption ''
