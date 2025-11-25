//! Modulo per il monitoraggio delle statistiche della CPU del processo server
//!
//! Questo modulo raccoglie e logga l'utilizzo **del processo corrente** (il
//! binario del server) a intervalli configurabili. La misurazione è basata su
//! `sysinfo` e non raccoglie più la media globale della macchina né l'utilizzo
//! per core (scopo: isolare il consumo del processo dell'applicazione).

use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;
use sysinfo::{System, Pid, ProcessesToUpdate};
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
    /// Tempo di CPU utilizzato dal processo nell'intervallo (in secondi)
    pub cpu_time_seconds: f64,
    /// Percentuale media di utilizzo CPU nell'intervallo
    pub avg_cpu_percentage: f32,
    /// Memoria usata dal processo in MB
    pub memory_usage_mb: f64,
}

impl CpuStats {
    /// Formatta le statistiche come stringa per il logging
    pub fn format_for_log(&self) -> String {
        format!(
            "[{}] CPU Time Used: {:.3}s | Avg CPU Usage: {:.2}% | Memory: {:.2} MB",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.cpu_time_seconds,
            self.avg_cpu_percentage,
            self.memory_usage_mb
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
        if let Err(e) = initialize_log_file(path, config.interval_secs) {
            error!("Failed to initialize CPU log file: {}", e);
        }
    }

    let mut sys = System::new_all();
    let current_pid = Pid::from_u32(std::process::id());
    
    info!("Monitoring process with PID: {}", current_pid);

    let mut interval = time::interval(Duration::from_secs(config.interval_secs));

    // Salta il primo tick che avviene immediatamente
    interval.tick().await;

    loop {
        // Momento iniziale dell'intervallo
        let start_time = std::time::Instant::now();
        
        // Raccoglie campioni durante l'intervallo per calcolare la media
        // 1 campione ogni secondo per tutta la durata dell'intervallo
        let sample_count = config.interval_secs; // Es: 120 campioni in 2 minuti
        let mut cpu_samples = Vec::with_capacity(sample_count as usize);
        let mut memory_samples = Vec::with_capacity(sample_count as usize);

        for _ in 0..sample_count {
            sys.refresh_processes(ProcessesToUpdate::Some(&[current_pid]), true);
            tokio::time::sleep(Duration::from_millis(200)).await;

            if let Some(process) = sys.process(current_pid) {
                cpu_samples.push(process.cpu_usage());
                memory_samples.push(process.memory() as f64 / (1024.0 * 1024.0));
            }

            // Attende 1 secondo tra un campione e l'altro
            tokio::time::sleep(Duration::from_millis(800)).await; // 200ms + 800ms = 1000ms
        }

        let elapsed = start_time.elapsed().as_secs_f64();

        // Raccoglie le statistiche
        let stats = collect_cpu_stats(&cpu_samples, &memory_samples, elapsed);

        // Log su file
        if let Some(ref path) = config.log_file_path {
            if let Err(e) = log_to_file(path, &stats) {
                error!("Failed to write CPU stats to file: {}", e);
            }
        }

        // Log in tempo reale tramite tracing
        if config.enable_realtime_logging {
            info!(
                "CPU Stats - Time: {:.3}s | Avg Usage: {:.2}% | Memory: {:.2} MB",
                stats.cpu_time_seconds,
                stats.avg_cpu_percentage,
                stats.memory_usage_mb
            );
        }

        // Attende il prossimo intervallo
        interval.tick().await;
    }
}

/// Raccoglie le statistiche della CPU dai campioni raccolti
fn collect_cpu_stats(cpu_samples: &[f32], memory_samples: &[f64], elapsed_seconds: f64) -> CpuStats {
    // Calcola la percentuale media di utilizzo CPU nell'intervallo
    let avg_cpu_percentage = if !cpu_samples.is_empty() {
        cpu_samples.iter().sum::<f32>() / cpu_samples.len() as f32
    } else {
        0.0
    };

    // Calcola la memoria media usata
    let memory_usage_mb = if !memory_samples.is_empty() {
        memory_samples.iter().sum::<f64>() / memory_samples.len() as f64
    } else {
        0.0
    };

    // Calcola il TEMPO EFFETTIVO di CPU utilizzato
    // Formula: (avg_cpu_percentage / 100) * elapsed_seconds
    let cpu_time_seconds = (avg_cpu_percentage / 100.0) as f64 * elapsed_seconds;

    CpuStats {
        timestamp: chrono::Utc::now(),
        cpu_time_seconds,
        avg_cpu_percentage,
        memory_usage_mb,
    }
}

/// Inizializza il file di log con un header
/// Il file viene resettato ad ogni avvio del server (truncate)
fn initialize_log_file(path: &str, interval_secs: u64) -> std::io::Result<()> {
    // Crea/ricrea il file (truncate) ad ogni avvio
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)  // Resetta il file ad ogni avvio
        .open(path)?;

    writeln!(file, "=== CPU Time Usage Log ===")?;
    writeln!(file, "Started: {} (UTC Time)", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "Logging Interval: {} seconds", interval_secs)?;
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
            cpu_time_seconds: 2.456,
            avg_cpu_percentage: 2.05,
            memory_usage_mb: 256.78,
        };

        let formatted = stats.format_for_log();
        assert!(formatted.contains("2.456s"));
        assert!(formatted.contains("2.05%"));
        assert!(formatted.contains("256.78 MB"));
    }

    #[test]
    fn test_default_config() {
        let config = CpuMonitorConfig::default();
        assert_eq!(config.interval_secs, 120);
        assert_eq!(config.log_file_path, Some("cpu_stats.log".to_string()));
        assert_eq!(config.enable_realtime_logging, false);
    }
}
