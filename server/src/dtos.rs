use serde::{Deserialize, Serialize};
use crate::entities::{IdType, User};

// struct per gestire io col client
#[derive(Serialize, Deserialize, Debug)]
pub struct UserDTO {
	pub id: Option<IdType>,
	pub username: Option<String>,
	#[serde(skip_serializing)]
	pub password: Option<String>,
}

impl From<User> for UserDTO {
	fn from(value: User) -> Self {
		Self {
			id: Some(value.id),
			username: Some(value.username),
			password: None, // mai esposta al client!!!
		}
	}
}