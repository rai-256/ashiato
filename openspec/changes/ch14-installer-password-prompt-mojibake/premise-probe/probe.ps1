# CH14 premises.md row 9: throwaway probe (not product/test code; a starting point for tasks 3.1).
# Starts a child on a pseudo console (ConPTY) and writes the rendered output (VT stream) to -Out as raw bytes.
# Key point: set STARTF_USESTDHANDLES with all three std handles NULL; otherwise, when the parent's
# stdout is a pipe, the child inherits the pipe instead of attaching to the pseudo console.
# Keep this file ASCII-only: Windows PowerShell 5.1 reads BOM-less UTF-8 as the ANSI code page and a
# multibyte comment can swallow the following line.
param([string]$Command, [string]$Out)
$src = @"
using System;
using System.IO;
using System.Runtime.InteropServices;
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
  [DllImport("kernel32.dll")] static extern void ClosePseudoConsole(IntPtr h);
  public static int Run(string cmd, string outPath) {
    SafeFileHandle inR, inW, outR, outW;
    CreatePipe(out inR, out inW, IntPtr.Zero, 0); CreatePipe(out outR, out outW, IntPtr.Zero, 0);
    IntPtr hpc; int hr = CreatePseudoConsole(new COORD{X=120,Y=40}, inR, outW, 0, out hpc);
    if (hr != 0) { Console.WriteLine("CreatePseudoConsole hr=" + hr); return 90; }
    IntPtr size = IntPtr.Zero; InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref size);
    IntPtr list = Marshal.AllocHGlobal(size); InitializeProcThreadAttributeList(list, 1, 0, ref size);
    UpdateProcThreadAttribute(list, 0, (IntPtr)0x00020016, hpc, (IntPtr)IntPtr.Size, IntPtr.Zero, IntPtr.Zero);
    STARTUPINFOEX si = new STARTUPINFOEX(); si.si.cb = Marshal.SizeOf(typeof(STARTUPINFOEX)); si.attr = list; si.si.flags = 0x100;
    PI pi; if (!CreateProcessW(null, cmd, IntPtr.Zero, IntPtr.Zero, false, 0x00080000, IntPtr.Zero, "C:\\", ref si, out pi)) { Console.WriteLine("CreateProcess err=" + Marshal.GetLastWin32Error()); return 91; }
    var ms = new MemoryStream(); var fs = new FileStream(outR, FileAccess.Read);
    var t = new Thread(() => { var b = new byte[4096]; int n; try { while ((n = fs.Read(b, 0, b.Length)) > 0) lock(ms) ms.Write(b, 0, n); } catch {} }); t.IsBackground = true; t.Start();
    WaitForSingleObject(pi.hp, 30000); Thread.Sleep(1000);
    uint code; GetExitCodeProcess(pi.hp, out code); ClosePseudoConsole(hpc); Thread.Sleep(500);
    lock(ms) File.WriteAllBytes(outPath, ms.ToArray());
    return (int)code;
  }
}
"@
Add-Type -TypeDefinition $src
$rc = [Pty]::Run($Command, $Out)
Write-Output ("rc=" + $rc)
exit $rc
