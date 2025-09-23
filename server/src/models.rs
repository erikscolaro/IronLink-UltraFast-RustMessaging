use sqlx::{SqlitePool, Row};
use tokio;
use dotenv::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct User {
	pub id: i64,
	pub name: String,
}

pub struct UserRepo {
	pool: SqlitePool,
}

impl UserRepo {
	// crea e connette direttamente al DB dal DATABASE_URL
	pub async fn new() -> Result<Self, sqlx::Error> {
		dotenv().ok();
		let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
		let pool = SqlitePool::connect(&database_url).await?;
		Ok(Self { pool })
	}

	pub async fn get_by_id(&self, id: i64) -> Result<Option<User>, sqlx::Error> {
		let row = sqlx::query("SELECT id, name FROM users WHERE id = ?")
			.bind(id)
			.fetch_optional(&self.pool)
			.await?;
		Ok(row.map(|r| User { id: r.get(0), name: r.get(1) }))
	}

	pub async fn create(&self, user: User) -> Result<(), sqlx::Error> {
		sqlx::query("INSERT INTO users (id, name) VALUES (?, ?)")
			.bind(user.id)
			.bind(user.name)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
}