use dotenv::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub server_host: String,
    pub server_port: u16,
    pub max_connections: u32,
    pub connection_lifetime_secs: u64,
    pub app_env: String,
}

impl Config {
    /// Carica la configurazione dalle variabili d'ambiente
    /// Chiama dotenv() automaticamente
    pub fn from_env() -> Result<Self, String> {
        dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set in .env file".to_string())?;

        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| {
                eprintln!("WARNING: JWT_SECRET not set, using default (not secure for production!)");
                "un segreto meno bello".to_string()
            });

        let server_host = env::var("SERVER_HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|_| "Invalid SERVER_PORT: must be a number between 0-65535".to_string())?;

        let max_connections = env::var("MAX_DB_CONNECTIONS")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<u32>()
            .map_err(|_| "Invalid MAX_DB_CONNECTIONS: must be a positive number".to_string())?;

        let connection_lifetime_secs = env::var("DB_CONNECTION_LIFETIME_SECS")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .map_err(|_| "Invalid DB_CONNECTION_LIFETIME_SECS: must be a positive number".to_string())?;

        let app_env = env::var("APP_ENV")
            .unwrap_or_else(|_| "development".to_string());

        Ok(Config {
            database_url,
            jwt_secret,
            server_host,
            server_port,
            max_connections,
            connection_lifetime_secs,
            app_env,
        })
    }

    /// Stampa la configurazione (nascondendo i segreti)
    pub fn print_info(&self) {
        println!("   Server Configuration:");
        println!("   Environment: {}", self.app_env);
        println!("   Server Address: {}:{}", self.server_host, self.server_port);
        println!("   Database: {}", Self::mask_url(&self.database_url));
        println!("   Max DB Connections: {}", self.max_connections);
        println!("   Connection Lifetime: {}s", self.connection_lifetime_secs);
        println!("   JWT Secret: {}", if self.jwt_secret == "un segreto meno bello" { 
            "   USING DEFAULT (INSECURE!)" 
        } else { 
            "✓ Custom secret configured" 
        });
    }

    /// Maschera l'URL del database per il logging
    fn mask_url(url: &str) -> String {
        if let Some(at_pos) = url.find('@') {
            if let Some(scheme_end) = url.find("://") {
                let scheme = &url[..scheme_end + 3];
                let after_at = &url[at_pos..];
                return format!("{}***{}", scheme, after_at);
            }
        }
        "***".to_string()
    }
}
