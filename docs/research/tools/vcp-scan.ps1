<#
  vcpscan.ps1 - READ-ONLY. Reads the VCP codes a monitor declares in its DDC/CI capabilities string.
  Never writes a value to the monitor. Works in Windows PowerShell 5.1 and PowerShell 7.
#>
Add-Type -TypeDefinition @'
using System;
using System.Collections;
using System.Runtime.InteropServices;
using System.Text;
public static class VcpScan {
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
  public static ArrayList Run() {
    var res = new ArrayList();
    EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, delegate(IntPtr hm, IntPtr hdc, IntPtr r, IntPtr d) {
      uint n; if (!GetNumberOfPhysicalMonitorsFromHMONITOR(hm, out n) || n == 0) return true;
      var arr = new PHYS[n]; GetPhysicalMonitorsFromHMONITOR(hm, n, arr);
      foreach (var p in arr) {
        uint len; string caps = "";
        if (GetCapabilitiesStringLength(p.h, out len) && len > 0) {
          var sb = new StringBuilder((int)len);
          if (CapabilitiesRequestAndCapabilitiesReply(p.h, sb, len)) caps = sb.ToString();
        }
        res.Add("MONITOR\t" + p.desc);
        res.Add("CAPS\t" + caps);
        // vcp( ... ) section: top-level hex codes; sub-values in nested parentheses are skipped
        int i = caps.IndexOf("vcp(");
        if (i >= 0) {
          i += 4; int depth = 1; var tok = new StringBuilder();
          for (; i < caps.Length && depth > 0; i++) {
            char c = caps[i];
            if (c == '(') { depth++; continue; }
            if (c == ')') { depth--; continue; }
            if (depth != 1) continue;
            if (Uri.IsHexDigit(c)) tok.Append(c);
            else { if (tok.Length > 0) { Emit(res, p.h, tok.ToString()); tok.Length = 0; } }
          }
          if (tok.Length > 0) Emit(res, p.h, tok.ToString());
        }
        DestroyPhysicalMonitor(p.h);
      }
      return true;
    }, IntPtr.Zero);
    return res;
  }
  static void Emit(ArrayList res, IntPtr h, string hex) {
    byte code = Convert.ToByte(hex, 16);
    uint t, cur, max;
    if (GetVCPFeatureAndVCPFeatureReply(h, code, out t, out cur, out max))
      res.Add(string.Format("VCP\t{0:X2}\t{1}\t{2}\t{3}", code, cur, max, t == 0 ? "momentary" : "set"));
    else res.Add(string.Format("VCP\t{0:X2}\tERR\t\t", code));
  }
}
'@
[VcpScan]::Run() | ForEach-Object { $_ }
