//! Modulo per il monitoraggio delle statistiche della CPU del processo server
//!
//! Questo modulo raccoglie e logga l'utilizzo **del processo corrente** (il
//! binario del server) a intervalli configurabili. La misurazione è basata su
//! `sysinfo` e non raccoglie più la media globale della macchina né l'utilizzo
//! per core (scopo: isolare il consumo del processo dell'applicazione).

use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;
use sysinfo::{System, Pid};
use tokio::time;
use tracing::{info, error};

/// Configurazione per il monitoraggio della CPU
pub struct CpuMonitorConfig {
    /// Intervallo di logging in secondi
    pub interval_secs: u64,
    /// Path del file di log (opzionale)
    pub log_file_path: Option<String>,
    /// Abilita logging in tempo reale tramite tracing
    pub enable_realtime_logging: bool,
}

impl Default for CpuMonitorConfig {
    fn default() -> Self {
        Self {
            interval_secs: 120, // 2 minuti
            log_file_path: Some("cpu_stats.log".to_string()),
            enable_realtime_logging: false,
        }
    }
}

/// Statistiche della CPU del processo raccolte in un dato momento
#[derive(Debug, Clone)]
pub struct CpuStats {
    /// Timestamp della raccolta
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Utilizzo della CPU del processo corrente (percentuale)
    pub process_cpu_usage: f32,
}

impl CpuStats {
    /// Formatta le statistiche come stringa per il logging
    pub fn format_for_log(&self) -> String {
        format!(
            "[{}] Process CPU Usage: {:.2}%",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.process_cpu_usage
        )
    }
}

/// Task di monitoraggio della CPU che gira in background
///
/// Questa funzione crea un task tokio che:
/// 1. Raccoglie le statistiche della CPU ogni `interval_secs` secondi
/// 2. Logga le statistiche su file (se configurato)
/// 3. Logga le statistiche in tempo reale tramite tracing (se abilitato)
///
/// # Arguments
///
/// * `config` - Configurazione del monitoraggio
///
/// # Example
///
/// ```rust,no_run
/// use server::monitoring::{start_cpu_monitoring, CpuMonitorConfig};
///
/// #[tokio::main]
/// async fn main() {
///     let config = CpuMonitorConfig {
///         interval_secs: 120,
///         log_file_path: Some("cpu_stats.log".to_string()),
///         enable_realtime_logging: false,
///     };
///     
///     tokio::spawn(start_cpu_monitoring(config));
/// }
/// ```
pub async fn start_cpu_monitoring(config: CpuMonitorConfig) {
    info!("Starting process CPU monitoring with interval: {} seconds", config.interval_secs);

    if let Some(ref path) = config.log_file_path {
        info!("Process CPU stats will be logged to: {}", path);
        if let Err(e) = initialize_log_file(path) {
            error!("Failed to initialize CPU log file: {}", e);
        }
    }

    let mut sys = System::new_all();
    let mut interval = time::interval(Duration::from_secs(config.interval_secs));

    // Salta il primo tick che avviene immediatamente
    interval.tick().await;

    // Recupera il pid del processo corrente (se disponibile)
    let maybe_pid: Option<Pid> = sysinfo::get_current_pid().ok();

    loop {
        interval.tick().await;

        // Per ottenere valori affidabili aggiorniamo le informazioni di sistema
        sys.refresh_all();

        // Breve attesa per stabilizzare le letture
        tokio::time::sleep(Duration::from_millis(200)).await;
        sys.refresh_all();

        // Raccoglie l'utilizzo CPU del processo corrente
        let process_cpu = if let Some(pid) = maybe_pid {
            sys.process(pid).map(|p| p.cpu_usage()).unwrap_or(0.0)
        } else {
            0.0
        };

        let stats = CpuStats {
            timestamp: chrono::Utc::now(),
            process_cpu_usage: process_cpu,
        };

        // Log su file
        if let Some(ref path) = config.log_file_path {
            if let Err(e) = log_to_file(path, &stats) {
                error!("Failed to write CPU stats to file: {}", e);
            }
        }

        // Log in tempo reale tramite tracing
        if config.enable_realtime_logging {
            info!("Process CPU Stats - Usage: {:.2}%", stats.process_cpu_usage);
        }
    }
}

/// Raccoglie le statistiche correnti della CPU

fn _collect_cpu_stats(_sys: &System) -> CpuStats {
    // funzione usata come placeholder
    CpuStats {
        timestamp: chrono::Utc::now(),
        process_cpu_usage: 0.0,
    }
}

/// Inizializza il file di log con un header
/// Il file viene resettato ad ogni avvio del server (truncate)
fn initialize_log_file(path: &str) -> std::io::Result<()> {
    // Crea/ricrea il file (truncate) ad ogni avvio
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)  // Resetta il file ad ogni avvio
        .open(path)?;

    writeln!(file, "=== CPU Statistics Log (Process) ===")?;
    writeln!(file, "Started: {} (UTC Time)", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "========================================\n")?;
    file.flush()?;

    Ok(())
}

/// Scrive le statistiche CPU su file
fn log_to_file(path: &str, stats: &CpuStats) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(path)?;

    writeln!(file, "{}", stats.format_for_log())?;
    file.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_stats_format() {
        let stats = CpuStats {
            timestamp: chrono::Utc::now(),
            process_cpu_usage: 45.67,
        };

        let formatted = stats.format_for_log();
        assert!(formatted.contains("45.67%"));
    }

    #[test]
    fn test_default_config() {
        let config = CpuMonitorConfig::default();
        assert_eq!(config.interval_secs, 120);
        assert_eq!(config.log_file_path, Some("cpu_stats.log".to_string()));
        assert_eq!(config.enable_realtime_logging, false);
    }
}
