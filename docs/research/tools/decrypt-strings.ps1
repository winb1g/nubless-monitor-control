param($bin, $tokenCsv, $tokenFile, $outFile, $asmName = 'SwitchAppModel.dll', $decTok = '0x06000071')
$ErrorActionPreference='Stop'
$script:loading = @{}
$selfName = [IO.Path]::GetFileNameWithoutExtension($asmName)
[AppDomain]::CurrentDomain.add_AssemblyResolve({ param($s,$e)
  $n = ($e.Name -split ',')[0]
  if ($n -eq $selfName -or $script:loading[$n]) { return $null }
  $script:loading[$n] = $true
  $p = Join-Path $bin "$n.dll"
  if (-not (Test-Path $p)) { $p = Join-Path $bin "$n.exe" }
  if (Test-Path $p) { return [Reflection.Assembly]::LoadFrom($p) } return $null })
$path = Join-Path $bin $asmName
$asm = [Reflection.Assembly]::LoadFrom($path)
$m = $asm.ManifestModule.ResolveMethod([Convert]::ToInt32($decTok.Substring(2),16))
$toks = if ($tokenFile) { Get-Content $tokenFile } else { $tokenCsv -split ',' }
$sb = New-Object System.Text.StringBuilder
foreach ($t in $toks) {
  try { $r = $m.Invoke($null, @([int]$t)); $r = ($r -replace "`r","\r") -replace "`n","\n"; [void]$sb.AppendLine(("{0}`t{1}" -f $t, $r)) } catch { [void]$sb.AppendLine(("{0}`tERR" -f $t)) }
}
if ($outFile) { [IO.File]::WriteAllText($outFile, $sb.ToString(), [Text.Encoding]::UTF8) } else { $sb.ToString() }
