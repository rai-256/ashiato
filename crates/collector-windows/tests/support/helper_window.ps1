# 実行時テストの相手役の窓（tests/runtime_windows.rs が起動する）。
# WinForms の窓を 1 つ出し、標準入力の 1 行ずつを命令として受ける:
#   title <文字列>   題名を変える
#   activate         前景にする
#   input            自分の窓へ無害なキー（F16）を 1 つ送る（最後の入力の時刻を「いま」にする）
#   quit             閉じて終わる
# 返事は標準出力に 1 行（ready <HWND> / ok）。
# **メッセージポンプを止めない** —— 標準入力を同期で待つと窓が応答せず、UI Automation の問い合わせが固まる
# （実測 2026-09-14: `[Console]::In.ReadLineAsync()` は同期で待つ実装なので固まった。
#   `OpenStandardInput()` の上に StreamReader を置けば、読み取りは別の糸で進む）。
# このファイルは UTF-8 BOM 付き（Windows PowerShell 5.1 は BOM が無いと ANSI として読む）。
param([string]$Title = "ashiato-rt")
Add-Type -AssemblyName System.Windows.Forms
$f = New-Object System.Windows.Forms.Form
$f.Text = $Title
$f.Width = 480
$f.Height = 240
$f.StartPosition = "Manual"
$f.Left = 40
$f.Top = 40
$f.Show()
$f.Activate()
[System.Windows.Forms.Application]::DoEvents()
[Console]::Out.WriteLine("ready " + $f.Handle)
[Console]::Out.Flush()
$in = New-Object System.IO.StreamReader([Console]::OpenStandardInput())
$task = $in.ReadLineAsync()
while ($true) {
  [System.Windows.Forms.Application]::DoEvents()
  if ($task.IsCompleted) {
    $line = $task.Result
    if ($null -eq $line) { break }
    $line = $line.Trim()
    if ($line -eq "quit") { break }
    elseif ($line -eq "activate") { $f.Activate(); $f.BringToFront() }
    elseif ($line -eq "input") { $f.Activate(); [System.Windows.Forms.SendKeys]::SendWait("{F16}") }
    elseif ($line.StartsWith("title ")) { $f.Text = $line.Substring(6) }
    [System.Windows.Forms.Application]::DoEvents()
    [Console]::Out.WriteLine("ok")
    [Console]::Out.Flush()
    $task = $in.ReadLineAsync()
  }
  Start-Sleep -Milliseconds 15
}
$f.Close()
exit 0
