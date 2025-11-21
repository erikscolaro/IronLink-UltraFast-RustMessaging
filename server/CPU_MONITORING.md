# CPU Monitoring

Il server include un sistema di monitoraggio delle statistiche della CPU che registra automaticamente i dati su file.

## Funzionalità

- **Logging automatico**: Le statistiche vengono raccolte ogni 2 minuti
- **File di log**: I dati vengono scritti in `cpu_stats.log` nella directory del server
- **Statistiche raccolte**:
  - Utilizzo globale della CPU (percentuale media)
  - Numero di core CPU
  - Utilizzo per ogni singolo core

## Formato del log

Il file `cpu_stats.log` contiene:

```
=== CPU Statistics Log ===
Started: 2025-11-18 10:30:45
========================================

[2025-11-18 10:32:45] Global CPU Usage: 23.45% | Cores: 8 | Per-core: [CPU0: 20.00%, CPU1: 25.00%, CPU2: 22.00%, CPU3: 24.00%, CPU4: 21.00%, CPU5: 26.00%, CPU6: 23.00%, CPU7: 27.00%]
[2025-11-18 10:34:45] Global CPU Usage: 18.32% | Cores: 8 | Per-core: [CPU0: 15.00%, CPU1: 20.00%, ...]
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

Per vedere le statistiche CPU anche nei log del server in tempo reale, modifica:

```rust
enable_realtime_logging: true,
```

Con questa opzione attiva, vedrai messaggi come:

```
2025-11-18T10:32:45.123Z INFO server: CPU Stats - Global: 23.45%, Cores: 8, Per-core: ["20.00%", "25.00%", ...]
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
- Le statistiche per core sono particolarmente utili per identificare squilibri nel carico di lavoro
- Il sistema usa la libreria `sysinfo` per raccogliere i dati in modo cross-platform
