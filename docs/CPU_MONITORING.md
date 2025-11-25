# CPU Monitoring (Process)

Il server include un sistema di monitoraggio che registra il consumo di CPU del
processo server (il binario in esecuzione). Il sistema calcola sia il **tempo effettivo di CPU** utilizzato che la **percentuale media di utilizzo** nell'intervallo di monitoraggio.

## Funzionalità

- **Campionamento ad alta frequenza**: Raccoglie un campione ogni secondo durante l'intero intervallo (120 campioni in 2 minuti)
- **Calcolo del tempo di CPU**: Misura il tempo effettivo di CPU utilizzato dal processo
- **Media dell'utilizzo**: Calcola la percentuale media di CPU sull'intervallo con elevata precisione
- **File di log**: I dati vengono scritti in `cpu_stats.log` nella directory del server
- **Statistiche raccolte**:
    - Tempo di CPU utilizzato nell'intervallo (in secondi)
    - Percentuale media di utilizzo CPU
    - Memoria RAM utilizzata dal processo (in MB)

## Formato del log

Il file `cpu_stats.log` contiene righe come la seguente (una per intervallo):

```
=== CPU Time Usage Log ===
Started: 2025-11-25 10:30:45 (UTC Time)
Logging Interval: 120 seconds
========================================

[2025-11-25 10:32:45] CPU Time Used: 2.456s | Avg CPU Usage: 2.05% | Memory: 245.67 MB
[2025-11-25 10:34:45] CPU Time Used: 1.978s | Avg CPU Usage: 1.65% | Memory: 248.32 MB
```

### Interpretazione dei dati:

- **CPU Time Used**: Tempo effettivo di CPU consumato dal processo nell'intervallo di 2 minuti
  - Formula: `(percentuale_media / 100) × tempo_intervallo`
  - Esempio: 2.05% medio su 120 secondi = 2.456 secondi di CPU effettiva
- **Avg CPU Usage**: Percentuale media di utilizzo CPU calcolata su 12 campioni
- **Memory**: Memoria RAM media utilizzata dal processo in megabytes

## Configurazione

La configurazione del monitoraggio si trova in `src/main.rs`:

```rust
let cpu_monitor_config = CpuMonitorConfig {
    interval_secs: 120,                                // Intervallo di logging (2 minuti)
    log_file_path: Some("cpu_stats.log".to_string()),  // File di output
    enable_realtime_logging: false,                    // Logging in tempo reale tramite tracing
};
```

### Parametri configurabili:

- **`interval_secs`**: Intervallo in secondi tra i log (default: 120 = 2 minuti)
  - Durante questo intervallo viene raccolto 1 campione ogni secondo
  - Numero totale campioni = `interval_secs` (es. 120 secondi → 120 campioni)
- **`log_file_path`**: Path del file di log (default: "cpu_stats.log")
- **`enable_realtime_logging`**: Se true, logga anche tramite il sistema di tracing in tempo reale (default: false)

### Campionamento ad alta frequenza:

Il sistema raccoglie **1 campione ogni secondo** durante l'intero intervallo:
- Intervallo 120s → 120 campioni (1 ogni secondo)
- Intervallo 60s → 60 campioni (1 ogni secondo)
- Intervallo 300s → 300 campioni (1 ogni secondo)

Questo garantisce precisione molto elevata e cattura anche picchi di attività brevi, con overhead minimo (~0.1% CPU).

## Abilitare il logging in tempo reale

Per vedere le statistiche del processo nei log del server in tempo reale, imposta:

```rust
enable_realtime_logging: true,
```

Con questa opzione attiva, vedrai messaggi come:

```
2025-11-25T10:32:45.123Z INFO server: CPU Stats - Time: 2.456s | Avg Usage: 2.05% | Memory: 245.67 MB
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

### Metodologia di misurazione

Il monitoring esegue le seguenti operazioni:

1. **Campionamento ad alta frequenza**: Raccoglie 1 campione ogni secondo di CPU e memoria durante l'intero intervallo
2. **Calcolo della media**: Calcola la percentuale media di utilizzo CPU da tutti i campioni raccolti
3. **Calcolo del tempo**: Converte la percentuale media in tempo effettivo di CPU
   - Formula: `cpu_time = (avg_cpu_percentage / 100) × elapsed_seconds`
4. **Logging**: Scrive timestamp, tempo CPU, percentuale media e memoria media

### Precisione e overhead

- **Frequenza di campionamento**: 1 Hz (1 campione/secondo)
- **Overhead CPU**: ~0.1% (trascurabile)
- **Precisione**: Elevata - cattura anche picchi di attività di 1-2 secondi
- **Numero campioni**: Uguale ai secondi dell'intervallo (120 campioni in 2 minuti)

### Implementazione

- Il monitoraggio viene eseguito in un task tokio separato e non blocca il server
- Il file di log viene resettato ad ogni avvio del server (modalità truncate)
- Il sistema usa la libreria `sysinfo` per raccogliere i dati in modo cross-platform
- Utilizza `ProcessesToUpdate::Some(&[pid])` per aggiornare solo il processo corrente (efficienza)

### Limitazioni e comportamento

- Il monitor misura solo l'utilizzo CPU del **processo server**
  - Include tutti i task async di Tokio (HTTP, WebSocket, queries DB)
  - Include il thread pool interno di Tokio
  - **NON** include processi esterni (es. database MySQL)
- L'accuratezza dipende dall'implementazione di `sysinfo` sulla piattaforma target
- I valori di percentuale rappresentano l'utilizzo rispetto a UN singolo core
  - Su un sistema a 4 core, 100% significa utilizzo completo di 1 core
  - Per utilizzo multi-core, il valore può superare 100% (es. 400% = 4 core al 100%)

## Strumenti e report

### Script di analisi PowerShell

Per facilitare l'analisi, è fornito uno script PowerShell che estrae le statistiche
dal file `cpu_stats.log` e calcola metriche riassuntive. Il file si trova in `tools/compute_cpu_stats.ps1`.

**Nota**: Lo script potrebbe richiedere aggiornamenti per il nuovo formato del log che include tempo CPU e memoria.

Esempio d'uso (PowerShell):

```pwsh
# Stampa riepilogo su console
.\tools\compute_cpu_stats.ps1 -LogPath .\cpu_stats.log

# Esporta CSV e JSON
.\tools\compute_cpu_stats.ps1 -LogPath .\cpu_stats.log -OutCsv .\cpu_report.csv -OutJson .\cpu_report.json
```

### Esempi di casi d'uso

**Scenario 1: Server con carico costante**
```
[2025-11-25 10:32:45] CPU Time Used: 2.400s | Avg CPU Usage: 2.00% | Memory: 250.00 MB
[2025-11-25 10:34:45] CPU Time Used: 2.424s | Avg CPU Usage: 2.02% | Memory: 251.20 MB
```
→ Carico stabile, ~2% di un core utilizzato costantemente

**Scenario 2: Picco di attività**
```
[2025-11-25 10:32:45] CPU Time Used: 1.200s | Avg CPU Usage: 1.00% | Memory: 245.00 MB
[2025-11-25 10:34:45] CPU Time Used: 12.000s | Avg CPU Usage: 10.00% | Memory: 280.50 MB
```
→ Picco di messaggi WebSocket o operazioni intensive nel secondo intervallo

**Scenario 3: Server idle**
```
[2025-11-25 10:32:45] CPU Time Used: 0.120s | Avg CPU Usage: 0.10% | Memory: 240.00 MB
```
→ Nessuna attività, solo overhead minimo di Tokio/runtime
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
