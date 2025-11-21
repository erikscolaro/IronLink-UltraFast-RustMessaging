//! Modulo per il monitoraggio delle statistiche della CPU
//!
//! Questo modulo fornisce funzionalità per raccogliere e loggare
//! le statistiche di utilizzo della CPU del server.

use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;
use sysinfo::System;
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

/// Statistiche della CPU raccolte in un dato momento
#[derive(Debug, Clone)]
pub struct CpuStats {
    /// Timestamp della raccolta
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Utilizzo globale della CPU in percentuale
    pub global_cpu_usage: f32,
    /// Numero di core CPU
    pub cpu_count: usize,
    /// Utilizzo per core (opzionale)
    pub per_core_usage: Vec<f32>,
}

impl CpuStats {
    /// Formatta le statistiche come stringa per il logging
    pub fn format_for_log(&self) -> String {
        let per_core = if !self.per_core_usage.is_empty() {
            format!(
                " | Per-core: [{}]",
                self.per_core_usage
                    .iter()
                    .enumerate()
                    .map(|(i, usage)| format!("CPU{}: {:.2}%", i, usage))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            String::new()
        };

        format!(
            "[{}] Global CPU Usage: {:.2}% | Cores: {}{}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.global_cpu_usage,
            self.cpu_count,
            per_core
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
    info!(
        "Starting CPU monitoring with interval: {} seconds",
        config.interval_secs
    );

    if let Some(ref path) = config.log_file_path {
        info!("CPU stats will be logged to: {}", path);
        
        // Crea il file se non esiste e scrivi l'header
        if let Err(e) = initialize_log_file(path) {
            error!("Failed to initialize CPU log file: {}", e);
        }
    }

    let mut sys = System::new_all();
    let mut interval = time::interval(Duration::from_secs(config.interval_secs));

    // Salta il primo tick che avviene immediatamente
    interval.tick().await;

    loop {
        interval.tick().await;

        // Aggiorna le informazioni di sistema
        sys.refresh_cpu_usage();
        
        // Aspetta un po' per avere letture accurate (raccomandazione di sysinfo)
        tokio::time::sleep(Duration::from_millis(200)).await;
        sys.refresh_cpu_usage();
        
        // Raccoglie le statistiche
        let stats = collect_cpu_stats(&sys);

        // Log su file
        if let Some(ref path) = config.log_file_path {
            if let Err(e) = log_to_file(path, &stats) {
                error!("Failed to write CPU stats to file: {}", e);
            }
        }

        // Log in tempo reale tramite tracing
        if config.enable_realtime_logging {
            info!(
                "CPU Stats - Global: {:.2}%, Cores: {}, Per-core: {:?}",
                stats.global_cpu_usage,
                stats.cpu_count,
                stats.per_core_usage
                    .iter()
                    .map(|u| format!("{:.2}%", u))
                    .collect::<Vec<_>>()
            );
        }
    }
}

/// Raccoglie le statistiche correnti della CPU
fn collect_cpu_stats(sys: &System) -> CpuStats {
    let cpus = sys.cpus();
    let cpu_count = cpus.len();
    
    // Calcola l'utilizzo medio globale
    let global_cpu_usage = if cpu_count > 0 {
        cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpu_count as f32
    } else {
        0.0
    };

    // Raccoglie l'utilizzo per ogni core
    let per_core_usage: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();

    CpuStats {
        timestamp: chrono::Utc::now(),
        global_cpu_usage,
        cpu_count,
        per_core_usage,
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

    writeln!(file, "=== CPU Statistics Log ===")?;
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
            global_cpu_usage: 45.67,
            cpu_count: 4,
            per_core_usage: vec![40.0, 50.0, 45.0, 48.0],
        };

        let formatted = stats.format_for_log();
        assert!(formatted.contains("45.67%"));
        assert!(formatted.contains("Cores: 4"));
        assert!(formatted.contains("CPU0: 40.00%"));
    }

    #[test]
    fn test_default_config() {
        let config = CpuMonitorConfig::default();
        assert_eq!(config.interval_secs, 120);
        assert_eq!(config.log_file_path, Some("cpu_stats.log".to_string()));
        assert_eq!(config.enable_realtime_logging, false);
    }
}
