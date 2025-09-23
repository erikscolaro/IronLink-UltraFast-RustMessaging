use dotenv::dotenv;
use std::env;

pub fn init() {
	dotenv().ok();
	println!("Config loaded: {:?}", env::var("APP_ENV").unwrap_or("dev".into()));
}

