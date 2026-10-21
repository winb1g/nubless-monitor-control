<#
.SYNOPSIS
  Asks LG's firmware version endpoint for the latest firmware file of each model in a list, the same URL LG Switch builds.
  One GET per model, sequential, with a delay. Writes a TSV: model, url, status, firmware file, Last-Modified.
.PARAMETER ModelsFile
  TSV with columns: model <TAB> family-suffix (may be empty) <TAB> source (ignored)
.NOTES
  Be polite: the default delay is 400 ms, and this should be run rarely. An unknown path returns HTTP 200 with the body
  "NotAllowedPage"; that is reported as "none".
#>
param(
  [Parameter(Mandatory)] [string]$ModelsFile,
  [Parameter(Mandatory)] [string]$OutFile,
  [int]$DelayMs = 400
)
$base = 'https://lmu.lge.com/ExternalService/onscreencontrol/fw/'
$rows = New-Object System.Collections.Generic.List[string]
$rows.Add("model`turl`tresult`tfirmware_file`tlast_modified")
foreach ($line in Get-Content $ModelsFile) {
  $p = $line -split "`t"
  $model = $p[0]; $suffix = if ($p.Count -gt 1) { $p[1] } else { '' }
  if (-not $model) { continue }
  $url = "$base$model$suffix/FWLatestVersion.txt"
  $raw = & curl.exe -s -m 20 -A 'nubless-monitor-control research (read-only)' -D - $url 2>$null
  $text = ($raw -join "`n")
  $status = if ($text -match '^HTTP/\S+\s+(\d{3})') { $Matches[1] } else { 'ERR' }
  $lm = if ($text -match '(?im)^last-modified:\s*(.+)$') { $Matches[1].Trim() } else { '' }
  $body = ($text -split "`r?`n`r?`n", 2)[-1].Trim()
  if ($body -match '^MODEL_\S+\.zip$') { $result = 'ok'; $file = $body } else { $result = 'none'; $file = '' }
  $rows.Add("$model`t$url`t$status/$result`t$file`t$lm")
  Start-Sleep -Milliseconds $DelayMs
}
[IO.File]::WriteAllLines($OutFile, $rows)
Write-Host "Wrote $($rows.Count - 1) rows to $OutFile"
