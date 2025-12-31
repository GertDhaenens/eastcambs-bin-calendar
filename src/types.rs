use std::sync::{Arc, Mutex};

pub use chrono::NaiveDate;
pub use serde::{Deserialize, Serialize};
pub use serde_json;

pub struct UserState {
    pub state: Option<String>,
    pub code: Option<String>,
}

impl UserState {
    pub fn new() -> Self {
        Self {
            state: None,
            code: None,
        }
    }
}

#[derive(Clone)]
pub struct ClientState {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub user_state: Arc<Mutex<UserState>>,
}

impl ClientState {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id: client_id,
            client_secret: client_secret,
            redirect_uri: redirect_uri,
            user_state: Arc::new(Mutex::new(UserState::new())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BagType {
    Black,
    Blue,
    GreenOrBrown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionDate {
    pub bag_type: BagType,
    pub date: NaiveDate,
}
