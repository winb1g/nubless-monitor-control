<#
.SYNOPSIS
  READ-ONLY: read specific VCP codes (declared or not) from a monitor over DDC/CI, with retries.
.EXAMPLE
  .\vcp-read.ps1 -Codes 0x15,0x4D,0x4E,0x4F,0xF7,0xF8,0xF9
.NOTES
  Uses only GetVCPFeatureAndVCPFeatureReply. Never sets anything. Undeclared codes usually answer
  "unsupported" (ERR). Output: code, current, max, and current in hex.
#>
param(
  [Parameter(Mandatory)] [string[]]$Codes,
  [int]$Monitor = 0,
  [int]$Retries = 3
)

Add-Type -TypeDefinition @'
using System;
using System.Collections;
using System.Runtime.InteropServices;
using System.Threading;
public static class VcpRead {
  delegate bool MonitorEnumProc(IntPtr hMon, IntPtr hdc, IntPtr rect, IntPtr data);
  [DllImport("user32.dll")] static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, MonitorEnumProc cb, IntPtr data);
  [DllImport("dxva2.dll")] static extern bool GetNumberOfPhysicalMonitorsFromHMONITOR(IntPtr h, out uint n);
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  struct PHYS { public IntPtr h; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string desc; }
  [DllImport("dxva2.dll")] static extern bool GetPhysicalMonitorsFromHMONITOR(IntPtr h, uint n, [Out] PHYS[] arr);
  [DllImport("dxva2.dll")] static extern bool DestroyPhysicalMonitor(IntPtr h);
  [DllImport("dxva2.dll")] static extern bool GetVCPFeatureAndVCPFeatureReply(IntPtr h, byte code, out uint type, out uint cur, out uint max);
  public static ArrayList Run(byte[] codes, int index, int retries) {
    var res = new ArrayList(); int seen = 0;
    EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, delegate(IntPtr hm, IntPtr hdc, IntPtr r, IntPtr d) {
      uint n; if (!GetNumberOfPhysicalMonitorsFromHMONITOR(hm, out n) || n == 0) return true;
      var arr = new PHYS[n]; GetPhysicalMonitorsFromHMONITOR(hm, n, arr);
      foreach (var p in arr) {
        if (seen++ == index) {
          foreach (byte c in codes) {
            string row = null;
            for (int a = 0; a < retries && row == null; a++) {
              uint t, cur, max;
              if (GetVCPFeatureAndVCPFeatureReply(p.h, c, out t, out cur, out max)) row = string.Format("0x{0:X2}\t{1}\t{2}\t0x{1:X}", c, cur, max);
              else Thread.Sleep(60);
            }
            res.Add(row ?? string.Format("0x{0:X2}\tERR (unsupported or no answer)", c));
            Thread.Sleep(50);
          }
        }
        DestroyPhysicalMonitor(p.h);
      }
      return true;
    }, IntPtr.Zero);
    return res;
  }
}
'@

# `powershell -File x.ps1 -Codes 0x15,0x4D` delivers one string, so split on commas ourselves.
$bytes = [byte[]]($Codes | ForEach-Object { $_ -split '[,\s]+' } | Where-Object { $_ } | ForEach-Object { [Convert]::ToByte(($_ -replace '^0x',''), 16) })
"code`tcur`tmax`tcur(hex)"
[VcpRead]::Run($bytes, $Monitor, $Retries) | ForEach-Object { $_ }
