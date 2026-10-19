# Read-only check of your monitor's firmware over DDC/CI (VCP code 0xC9, "Display firmware level").
# It only READS. It never writes anything to the monitor. Works in Windows PowerShell 5.1 and PowerShell 7.
# Tip: run it while the monitor is connected directly by DisplayPort/HDMI (docks and KVMs can block DDC/CI).
& {
  if (-not ('FwCheck' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections;
using System.Runtime.InteropServices;
public static class FwCheck {
  delegate bool EnumProc(IntPtr hMon, IntPtr hdc, IntPtr rect, IntPtr data);
  [DllImport("user32.dll")] static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, EnumProc cb, IntPtr data);
  [DllImport("dxva2.dll")] static extern bool GetNumberOfPhysicalMonitorsFromHMONITOR(IntPtr h, out uint n);
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  struct PHYS { public IntPtr h; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string desc; }
  [DllImport("dxva2.dll")] static extern bool GetPhysicalMonitorsFromHMONITOR(IntPtr h, uint n, [Out] PHYS[] arr);
  [DllImport("dxva2.dll")] static extern bool DestroyPhysicalMonitor(IntPtr h);
  [DllImport("dxva2.dll")] static extern bool GetVCPFeatureAndVCPFeatureReply(IntPtr h, byte code, out uint type, out uint cur, out uint max);
  public static ArrayList Run() {
    var res = new ArrayList();
    EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, delegate(IntPtr hm, IntPtr hdc, IntPtr r, IntPtr d) {
      uint n; if (!GetNumberOfPhysicalMonitorsFromHMONITOR(hm, out n) || n == 0) return true;
      var arr = new PHYS[n]; GetPhysicalMonitorsFromHMONITOR(hm, n, arr);
      foreach (var p in arr) {
        uint t, cur, max;
        if (GetVCPFeatureAndVCPFeatureReply(p.h, 0xC9, out t, out cur, out max))
          res.Add(string.Format("{0}.{1:00}  (raw 0x{2:X4})", cur >> 8, cur & 0xFF, cur));
        else res.Add("no answer (this display did not report a firmware level over DDC/CI)");
        DestroyPhysicalMonitor(p.h);
      }
      return true;
    }, IntPtr.Zero);
    return res;
  }
}
'@
  }

  $edid = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID -ErrorAction SilentlyContinue | ForEach-Object {
    $name = ($_.UserFriendlyName | Where-Object { $_ -ne 0 } | ForEach-Object { [char]$_ }) -join ''
    $code = ($_.ProductCodeID    | Where-Object { $_ -ne 0 } | ForEach-Object { [char]$_ }) -join ''
    '{0} (EDID product code {1}, made week {2} of {3})' -f $name, $code, $_.WeekOfManufacture, $_.YearOfManufacture
  })
  $fw = @([FwCheck]::Run())

  Write-Host ''
  Write-Host '=== Monitor firmware check (read-only) ==='
  Write-Host 'Monitors Windows can see:'
  if ($edid.Count) { $edid | ForEach-Object { Write-Host "  $_" } } else { Write-Host '  (none reported)' }
  Write-Host 'Display firmware level (VCP C9), in the same order:'
  if ($fw.Count) { $fw | ForEach-Object { Write-Host "  $_" } } else { Write-Host '  (no physical monitor answered)' }
  Write-Host ''
  Write-Host 'Enter the first number (for example 3.05) in the compare box on the page.'
}
