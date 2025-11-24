# CPU Monitoring (Process)

Il server include un sistema di monitoraggio che registra il consumo di CPU del
processo server (il binario in esecuzione). Per scelta progettuale questo file
mostra come il monitor misuri esclusivamente l'utilizzo del processo.

## Funzionalità

- **Logging automatico**: Le statistiche del processo vengono raccolte ogni 2 minuti 
- **File di log**: I dati vengono scritti in `cpu_stats.log` nella directory del server
- **Statistiche raccolte**:
    - Utilizzo della CPU del processo (percentuale)

## Formato del log

Il file `cpu_stats.log` contiene righe come la seguente (una per intervallo):

```
=== CPU Statistics Log (Process) ===
Started: 2025-11-18 10:30:45
========================================

[2025-11-18 10:32:45] Process CPU Usage: 3.45%
[2025-11-18 10:34:45] Process CPU Usage: 1.12%
```

## Configurazione

La configurazione del monitoraggio si trova in `src/main.rs`:

```rust
let cpu_monitor_config = CpuMonitorConfig {
    interval_secs: 120,                           // Intervallo di logging (2 minuti)
    log_file_path: Some("cpu_stats.log".to_string()), // File di output
    enable_realtime_logging: false,               // Logging in tempo reale tramite tracing
};
```

### Parametri configurabili:

- **`interval_secs`**: Intervallo in secondi tra le raccolte di statistiche (default: 120 = 2 minuti)
- **`log_file_path`**: Path del file di log (default: "cpu_stats.log")
- **`enable_realtime_logging`**: Se true, logga anche tramite il sistema di tracing in tempo reale (default: false)

## Abilitare il logging in tempo reale

Per vedere l'utilizzo del processo nei log del server in tempo reale, imposta:

```rust
enable_realtime_logging: true,
```

Con questa opzione attiva, vedrai messaggi come:

```
2025-11-18T10:32:45.123Z INFO server: Process CPU Stats - Usage: 3.45%
```

## Utilizzo del modulo

Il modulo `monitoring` può essere utilizzato anche programmaticamente:

```rust
use server::monitoring::{start_cpu_monitoring, CpuMonitorConfig};

#[tokio::main]
async fn main() {
    // Configurazione personalizzata
    let config = CpuMonitorConfig {
        interval_secs: 60,  // Ogni minuto
        log_file_path: Some("custom_cpu.log".to_string()),
        enable_realtime_logging: true,
    };
    
    // Avvia il task di monitoraggio
    tokio::spawn(start_cpu_monitoring(config));
    
    // Il resto dell'applicazione...
}
```

## Note tecniche

- Il monitoraggio viene eseguito in un task tokio separato e non blocca il server
- Il file di log viene aperto in modalità append, quindi i dati si accumulano tra un riavvio e l'altro
- Il sistema usa la libreria `sysinfo` per raccogliere i dati in modo cross-platform

### Limitazioni e comportamento

- Il monitor misura solo l'utilizzo CPU del processo server. Non fornisce metriche di sistema (globali) né per-core.
- L'accuratezza del valore dipende dall'implementazione di `sysinfo` sulla piattaforma target; i valori sono percentuali (0.0 - 100.0).

## Strumenti e report

Per facilitare l'analisi, è fornito uno script PowerShell che estrae le percentuali
dal file `cpu_stats.log` e calcola statistiche riassuntive (count, min, max,
average). Il file si trova in `tools/compute_cpu_stats.ps1`.

Esempio d'uso (PowerShell):

```pwsh
# Stampa riepilogo su console
.\n+tools\compute_cpu_stats.ps1 -LogPath .\cpu_stats.log

# Esporta CSV e JSON
.
tools\compute_cpu_stats.ps1 -LogPath .\cpu_stats.log -OutCsv .\cpu_report.csv -OutJson .\cpu_report.json
```

Esempio di output del report:

```
CPU process usage report for: .\cpu_stats.log

Count   : 11
Min     : 6.67
Max     : 66.67
Average : 34.84
Sum     : 383.24
FirstTimestamp : 2025-11-21 16:04:35
LastTimestamp  : 2025-11-21 16:24:35
```

Riferimenti pratici:

- `debug` exe (per sviluppo): `target/debug/server.exe` — dimensione rilevata: **16.01 MB**
- `release` exe (per distribuzione): `target/release/server.exe` — dimensione rilevata: **2.86 MB**

Se vuoi, posso aggiungere un piccolo task CI o una GitHub Action che produce automaticamente
questo report dopo ogni build sul branch `dev`.

### Misure di compilazione (locale)

- `Measure-Command { cargo build --release }` (misura eseguita nella workspace corrente): **0.61 s** (TotalMilliseconds: ~614.49)

Nota: il tempo riportato sopra è relativo a una build nella workspace dove gli artefatti erano aggiornati (build incrementale).

Esempio output di una build `--release` eseguita come *clean build* sulla stessa macchina (log troncato per brevità):

```text
PS C:\Users\User\OneDrive - University of Pisa\Polito\ANNO_1\2_SEM\Programmazione di sistema\RUGGINE\G43\server> cargo build --release
    Compiling proc-macro2 v1.0.101
    Compiling unicode-ident v1.0.19
    Compiling version_check v0.9.5
    ...
    Compiling sysinfo v0.32.1
    Compiling server v0.1.0 (C:\Users\User\OneDrive - University of Pisa\Polito\ANNO_1\2_SEM\Programmazione di sistema\RUGGINE\G43\server)
     Finished `release` profile [optimized] target(s) in 4m 12s
```

Misura (clean build): **4 minuti 12 secondi**. I tempi di compilazione variano in base a CPU, I/O, cache e configurazione della toolchain.
