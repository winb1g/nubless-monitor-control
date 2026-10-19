<#
.SYNOPSIS
  READ-ONLY snapshot/diff of every VCP code a monitor declares over DDC/CI.
  Use it to learn which VCP code a setting in the LG Switch "Screen Settings" panel (or the monitor's OSD) controls.

.EXAMPLE
  # 1) snapshot, 2) change ONE setting in LG Switch (or the OSD), 3) diff
  .\vcp-diff.ps1 -Save before.json
  #   ...change e.g. "Response Time" from Fast to Faster in the app...
  .\vcp-diff.ps1 -Compare before.json

.NOTES
  Never writes to the monitor. DDC/CI is slow and sometimes flaky: each code is retried, and a code that
  answered in one snapshot but not the other is reported as "unstable", not as a change.
  Works in Windows PowerShell 5.1 and PowerShell 7. Snapshots are saved only where you point -Save/-Out.
#>
param(
  [string]$Save,
  [string]$Compare,
  [int]$Monitor = 0,      # index among physical monitors that answer DDC/CI
  [int]$Retries = 3
)

Add-Type -TypeDefinition @'
using System;
using System.Collections;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
public static class VcpSnap {
  delegate bool MonitorEnumProc(IntPtr hMon, IntPtr hdc, IntPtr rect, IntPtr data);
  [DllImport("user32.dll")] static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, MonitorEnumProc cb, IntPtr data);
  [DllImport("dxva2.dll")] static extern bool GetNumberOfPhysicalMonitorsFromHMONITOR(IntPtr h, out uint n);
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  struct PHYS { public IntPtr h; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string desc; }
  [DllImport("dxva2.dll")] static extern bool GetPhysicalMonitorsFromHMONITOR(IntPtr h, uint n, [Out] PHYS[] arr);
  [DllImport("dxva2.dll")] static extern bool DestroyPhysicalMonitor(IntPtr h);
  [DllImport("dxva2.dll")] static extern bool GetVCPFeatureAndVCPFeatureReply(IntPtr h, byte code, out uint type, out uint cur, out uint max);
  [DllImport("dxva2.dll")] static extern bool GetCapabilitiesStringLength(IntPtr h, out uint len);
  [DllImport("dxva2.dll")] static extern bool CapabilitiesRequestAndCapabilitiesReply(IntPtr h, StringBuilder s, uint len);

  public static string Caps; public static ArrayList Rows;
  public static void Run(int index, int retries) {
    Rows = new ArrayList(); Caps = "";
    int seen = 0;
    EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, delegate(IntPtr hm, IntPtr hdc, IntPtr r, IntPtr d) {
      uint n; if (!GetNumberOfPhysicalMonitorsFromHMONITOR(hm, out n) || n == 0) return true;
      var arr = new PHYS[n]; GetPhysicalMonitorsFromHMONITOR(hm, n, arr);
      foreach (var p in arr) {
        if (seen++ == index) {
          uint len;
          if (GetCapabilitiesStringLength(p.h, out len) && len > 0) {
            var sb = new StringBuilder((int)len);
            if (CapabilitiesRequestAndCapabilitiesReply(p.h, sb, len)) Caps = sb.ToString();
          }
          foreach (byte code in Declared(Caps)) {
            string row = null;
            for (int a = 0; a < retries && row == null; a++) {
              uint t, cur, max;
              if (GetVCPFeatureAndVCPFeatureReply(p.h, code, out t, out cur, out max)) row = string.Format("{0:X2}\t{1}\t{2}", code, cur, max);
              else Thread.Sleep(60);
            }
            Rows.Add(row ?? string.Format("{0:X2}\tERR\t", code));
          }
        }
        DestroyPhysicalMonitor(p.h);
      }
      return true;
    }, IntPtr.Zero);
  }
  static ArrayList Declared(string caps) {
    var list = new ArrayList(); int i = caps.IndexOf("vcp("); if (i < 0) return list;
    i += 4; int depth = 1; var tok = new StringBuilder();
    for (; i < caps.Length && depth > 0; i++) {
      char c = caps[i];
      if (c == '(') { depth++; continue; }
      if (c == ')') { depth--; continue; }
      if (depth != 1) continue;
      if (Uri.IsHexDigit(c)) tok.Append(c); else if (tok.Length > 0) { list.Add(Convert.ToByte(tok.ToString(), 16)); tok.Length = 0; }
    }
    if (tok.Length > 0) list.Add(Convert.ToByte(tok.ToString(), 16));
    return list;
  }
}
'@

function Get-Snapshot {
  [VcpSnap]::Run($Monitor, $Retries)
  $codes = [ordered]@{}
  foreach ($row in [VcpSnap]::Rows) {
    $p = $row -split "`t"
    $codes[$p[0]] = if ($p[1] -eq 'ERR') { $null } else { @{ cur = [int]$p[1]; max = [int]$p[2] } }
  }
  [pscustomobject]@{ taken = (Get-Date).ToString('s'); caps = [VcpSnap]::Caps; codes = $codes }
}

$snap = Get-Snapshot
if (-not $snap.codes.Count) { Write-Error 'No VCP codes read. Is DDC/CI enabled, and is the monitor connected directly (not via a dock/KVM)?'; exit 1 }

if ($Save) {
  $snap | ConvertTo-Json -Depth 6 | Set-Content -Path $Save -Encoding UTF8
  Write-Host "Saved $($snap.codes.Count) codes to $Save. Now change ONE setting, then run:  .\vcp-diff.ps1 -Compare `"$Save`""
}
elseif ($Compare) {
  $old = Get-Content $Compare -Raw | ConvertFrom-Json
  $changed = @(); $unstable = @()
  foreach ($k in $snap.codes.Keys) {
    $a = $old.codes.$k; $b = $snap.codes[$k]
    if ($null -eq $a -or $null -eq $b) { if (($null -eq $a) -ne ($null -eq $b)) { $unstable += $k }; continue }
    if ($a.cur -ne $b.cur) { $changed += [pscustomobject]@{ VCP = "0x$k"; Before = $a.cur; After = $b.cur; BeforeHex = ('0x{0:X}' -f $a.cur); AfterHex = ('0x{0:X}' -f $b.cur); Max = $b.max } }
  }
  if ($changed) { Write-Host "Changed codes:"; $changed | Format-Table -AutoSize | Out-String | Write-Host }
  else { Write-Host 'No declared VCP code changed. (The setting may be software-side, or use a private opcode outside the declared list.)' }
  if ($unstable) { Write-Host ("Unstable (answered in only one snapshot, ignore or re-run): " + ($unstable -join ', ')) }
}
else {
  $snap.codes.GetEnumerator() | ForEach-Object { '{0}  cur={1}  max={2}' -f $_.Key, $(if ($_.Value) { $_.Value.cur } else { 'ERR' }), $(if ($_.Value) { $_.Value.max } else { '' }) }
}
