# conpty-run.ps1 -- run a command on a pseudo console (ConPTY) and save what the window shows.
#
# What: starts `cmd.exe /d /c "<Command>"` attached to a pseudo console (120x40), feeds lines from
#   -InputFile one at a time, and writes the rendered output (the VT stream, UTF-8) to -Out.
# Why (CH14): the installer's password prompt garbled on a real console window (code page 932).
#   A pipe hides that, so the test needs a real console. This script is that console, driven from
#   tools/release/install-check.sh. Japanese expected strings live on the bash side; this file stays
#   ASCII only (Windows PowerShell 5.1 reads BOM-less UTF-8 as the ANSI code page and a multibyte
#   comment can swallow the next line).
#
# Parameters:
#   -Command     string handed to `cmd.exe /d /c`.
#   -InputFile   file with one line to send per row (optional). A line is sent (plus CR) once the
#                window output has been quiet for 1 second.
#   -Out         file that receives the window output as UTF-8 bytes.
#   -TimeoutSec  upper limit in seconds (default 120). When exceeded the child is killed, rc=124.
#   -Probe       negative control. Runs two children in a `chcp 932` window, one after the other:
#                  raw     writes the UTF-8 bytes of the prompt text to the console output handle
#                          (the WriteFile path, same as rpassword) -> expected to garble;
#                  console writes the same text through [Console]::Out (the WriteConsoleW path).
#                Output goes to "<Out>.raw.txt" and "<Out>.console.txt". Both children first print
#                CONSOLE-OK when stdin/stdout are NOT redirected (the child really sits on the window).
#
# Exit code: the child's exit code (or 124 on timeout, 90/91 when the pseudo console cannot start).
param(
  [string]$Command,
  [string]$InputFile,
  [string]$Out,
  [int]$TimeoutSec = 120,
  [switch]$Probe
)
$src = @"
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using Microsoft.Win32.SafeHandles;
public static class Pty {
  [StructLayout(LayoutKind.Sequential)] public struct COORD { public short X; public short Y; }
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] public struct STARTUPINFO { public int cb; public string r; public string d; public string t; public int x; public int y; public int xs; public int ys; public int xc; public int yc; public int fa; public int flags; public short sw; public short cb2; public IntPtr r2; public IntPtr hin; public IntPtr hout; public IntPtr herr; }
  [StructLayout(LayoutKind.Sequential)] public struct STARTUPINFOEX { public STARTUPINFO si; public IntPtr attr; }
  [StructLayout(LayoutKind.Sequential)] public struct PI { public IntPtr hp; public IntPtr ht; public int pid; public int tid; }
  [DllImport("kernel32.dll")] static extern int CreatePseudoConsole(COORD s, SafeFileHandle i, SafeFileHandle o, uint f, out IntPtr h);
  [DllImport("kernel32.dll")] static extern bool CreatePipe(out SafeFileHandle r, out SafeFileHandle w, IntPtr a, int n);
  [DllImport("kernel32.dll", SetLastError=true)] static extern bool InitializeProcThreadAttributeList(IntPtr l, int c, int f, ref IntPtr s);
  [DllImport("kernel32.dll", SetLastError=true)] static extern bool UpdateProcThreadAttribute(IntPtr l, uint f, IntPtr a, IntPtr v, IntPtr s, IntPtr p, IntPtr r);
  [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)] static extern bool CreateProcessW(string a, string c, IntPtr pa, IntPtr ta, bool inh, uint f, IntPtr e, string d, ref STARTUPINFOEX si, out PI pi);
  [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr h, uint ms);
  [DllImport("kernel32.dll")] static extern bool GetExitCodeProcess(IntPtr h, out uint c);
  [DllImport("kernel32.dll")] static extern bool TerminateProcess(IntPtr h, uint c);
  [DllImport("kernel32.dll")] static extern void ClosePseudoConsole(IntPtr h);
  public static int Run(string cmd, string[] lines, string outPath, int timeoutSec) {
    SafeFileHandle inR, inW, outR, outW;
    CreatePipe(out inR, out inW, IntPtr.Zero, 0); CreatePipe(out outR, out outW, IntPtr.Zero, 0);
    IntPtr hpc; int hr = CreatePseudoConsole(new COORD{X=120,Y=40}, inR, outW, 0, out hpc);
    if (hr != 0) { Console.WriteLine("CreatePseudoConsole hr=" + hr); return 90; }
    IntPtr size = IntPtr.Zero; InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref size);
    IntPtr list = Marshal.AllocHGlobal(size); InitializeProcThreadAttributeList(list, 1, 0, ref size);
    UpdateProcThreadAttribute(list, 0, (IntPtr)0x00020016, hpc, (IntPtr)IntPtr.Size, IntPtr.Zero, IntPtr.Zero);
    // STARTF_USESTDHANDLES with three empty handles and bInheritHandles=false: otherwise, when this
    // process's stdout is a pipe (CI pipes install-check into tee), the child inherits that pipe
    // instead of attaching to the pseudo console.
    STARTUPINFOEX si = new STARTUPINFOEX(); si.si.cb = Marshal.SizeOf(typeof(STARTUPINFOEX)); si.attr = list; si.si.flags = 0x100;
    si.si.hin = IntPtr.Zero; si.si.hout = IntPtr.Zero; si.si.herr = IntPtr.Zero;
    PI pi; if (!CreateProcessW(null, cmd, IntPtr.Zero, IntPtr.Zero, false, 0x00080000, IntPtr.Zero, "C:\\", ref si, out pi)) { Console.WriteLine("CreateProcess err=" + Marshal.GetLastWin32Error()); return 91; }
    var ms = new MemoryStream(); var fs = new FileStream(outR, FileAccess.Read);
    long last = Environment.TickCount; bool any = false;
    var t = new Thread(() => { var b = new byte[4096]; int n; try { while ((n = fs.Read(b, 0, b.Length)) > 0) { lock(ms) { ms.Write(b, 0, n); last = Environment.TickCount; any = true; } } } catch {} }); t.IsBackground = true; t.Start();
    var inS = new FileStream(inW, FileAccess.Write);
    int next = 0; long deadline = Environment.TickCount + (long)timeoutSec * 1000; int rc;
    while (true) {
      if (WaitForSingleObject(pi.hp, 100) == 0) { uint code; GetExitCodeProcess(pi.hp, out code); rc = (int)code; break; }
      if (Environment.TickCount > deadline) { TerminateProcess(pi.hp, 124); rc = 124; break; }
      bool quiet; lock(ms) { quiet = any && (Environment.TickCount - last) >= 1000; }
      if (quiet && next < lines.Length) {
        var bytes = Encoding.UTF8.GetBytes(lines[next++] + "\r"); inS.Write(bytes, 0, bytes.Length); inS.Flush();
        lock(ms) { last = Environment.TickCount; }
      }
    }
    Thread.Sleep(1000); ClosePseudoConsole(hpc); Thread.Sleep(500);
    lock(ms) File.WriteAllBytes(outPath, ms.ToArray());
    return rc;
  }
}
"@
Add-Type -TypeDefinition $src

function Invoke-Pty([string]$cmdline, [string[]]$lines, [string]$outPath, [int]$limit) {
  return [Pty]::Run($cmdline, $lines, $outPath, $limit)
}

function New-ProbeCommand([string]$body) {
  # Child scripts are passed as -EncodedCommand so no quoting survives cmd.exe.
  $enc = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($body))
  return 'chcp 932 >nul & chcp & powershell.exe -NoProfile -NonInteractive -EncodedCommand ' + $enc
}

if ($Probe) {
  # The prompt text is built from code points so this file stays ASCII.
  $text = '$s = ([string][char]0x753B) + [char]0x9762 + [char]0x306E + [char]0x5408 + [char]0x8A00 + [char]0x8449 + ": "'
  $ok = 'if (-not [Console]::IsInputRedirected -and -not [Console]::IsOutputRedirected) { [Console]::Out.WriteLine("CONSOLE-OK") }'
  $rawBody = $ok + "`n" + $text + "`n" + '$b = [Text.Encoding]::UTF8.GetBytes($s); $o = [Console]::OpenStandardOutput(); $o.Write($b, 0, $b.Length); $o.Flush(); [Console]::Out.WriteLine()'
  $conBody = $ok + "`n" + $text + "`n" + '[Console]::Out.WriteLine($s)'
  $rc1 = Invoke-Pty ('cmd.exe /d /c "' + (New-ProbeCommand $rawBody) + '"') @() ($Out + '.raw.txt') $TimeoutSec
  $rc2 = Invoke-Pty ('cmd.exe /d /c "' + (New-ProbeCommand $conBody) + '"') @() ($Out + '.console.txt') $TimeoutSec
  if ($rc1 -ne 0) { exit $rc1 }
  exit $rc2
}

$lines = @()
if ($InputFile) { $lines = [IO.File]::ReadAllLines($InputFile, [Text.Encoding]::UTF8) }
$rc = Invoke-Pty ('cmd.exe /d /c "' + $Command + '"') $lines $Out $TimeoutSec
exit $rc
