# Child of probe.ps1: reports whether stdin/stdout are redirected and writes the prompt two ways (raw UTF-8 bytes via WriteFile / Console.Out = WriteConsoleW).
$s = [string]::new([char[]]@(0x753B,0x9762,0x306E,0x5408,0x8A00,0x8449)) + ": "
$b = [Text.Encoding]::UTF8.GetBytes("RAW[" + $s + "]`r`n")
$o = [Console]::OpenStandardOutput(); $o.Write($b, 0, $b.Length); $o.Flush()
[Console]::Out.WriteLine("CON[" + $s + "] REDIR in=" + [Console]::IsInputRedirected + " out=" + [Console]::IsOutputRedirected)
