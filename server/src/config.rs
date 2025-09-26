use dotenv::dotenv;
use std::env;
use std::env::VarError;

pub fn init() {
    dotenv().ok();

    println!(
        "Config loaded: {:?}",
        env::var("APP_ENV").unwrap_or("dev".into())
    );
}
