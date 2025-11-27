<#
.SYNOPSIS
  Analizza `cpu_stats.log` e calcola min/max/avg delle percentuali di CPU del processo.

.DESCRIPTION
  Estrae le righe con formato '[TIMESTAMP] Process CPU Usage: X%' dal file di log,
  calcola statistiche (count, min, max, average) e può esportare i risultati in
  CSV o JSON.

.PARAMETER LogPath
  Path del file di log. Default: .\cpu_stats.log

.PARAMETER OutCsv
  Path CSV di output (opzionale). Se omesso non viene scritto.

.PARAMETER OutJson
  Path JSON di output (opzionale). Se omesso non viene scritto.

.EXAMPLE
  .\compute_cpu_stats.ps1 -LogPath .\cpu_stats.log

  Esegue l'analisi e stampa le statistiche in console.
#>

param(
    [string]$LogPath = ".\cpu_stats.log",
    [string]$OutCsv = "",
    [string]$OutJson = ""
)

if (-not (Test-Path $LogPath)) {
    Write-Error "Log file not found: $LogPath"
    exit 2
}

$pattern = '^\[(?<ts>[^\]]+)\]\s+Process CPU Usage:\s+(?<val>[0-9]+(?:\.[0-9]+)?)%'

$entries = Get-Content $LogPath | ForEach-Object {
    if ($_ -match $pattern) {
        [PSCustomObject]@{
            Timestamp = [datetime]::ParseExact($matches['ts'], 'yyyy-MM-dd HH:mm:ss', $null)
                -as [datetime]
            Value = [double]$matches['val']
        }
    }
} | Where-Object { $_ -ne $null }

if (-not $entries) {
    Write-Output "No matching entries found in $LogPath"
    exit 0
}

$values = $entries | Select-Object -ExpandProperty Value
$stats = $values | Measure-Object -Minimum -Maximum -Average -Sum -Property *

$report = [PSCustomObject]@{
    Count = $stats.Count
    Min = [math]::Round($stats.Minimum, 2)
    Max = [math]::Round($stats.Maximum, 2)
    Average = [math]::Round($stats.Average, 2)
    Sum = [math]::Round($stats.Sum, 2)
    FirstTimestamp = ($entries | Select-Object -First 1).Timestamp
    LastTimestamp = ($entries | Select-Object -Last 1).Timestamp
}

Write-Output "CPU process usage report for: $LogPath"
Write-Output ($report | Format-List | Out-String)

if ($OutCsv -ne "") {
    $entries | Select-Object Timestamp, Value | Export-Csv -Path $OutCsv -NoTypeInformation -Encoding UTF8
    Write-Output "Wrote CSV: $OutCsv"
}

if ($OutJson -ne "") {
    $entries | ConvertTo-Json -Depth 3 | Out-File -FilePath $OutJson -Encoding UTF8
    Write-Output "Wrote JSON: $OutJson"
}

exit 0
