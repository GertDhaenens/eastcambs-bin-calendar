pub use chrono::NaiveDate;
pub use google_calendar;
pub use serde::{Deserialize, Serialize};
pub use serde_json;
pub use std::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,

    client: Option<google_calendar::Client>,
}

impl AppState {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id: client_id,
            client_secret: client_secret,
            redirect_uri: redirect_uri,
            client: None,
        }
    }

    pub fn get_client(&mut self) -> &google_calendar::Client {
        match self.client {
            None => {
                self.client = Some(google_calendar::Client::new(
                    &self.client_id,
                    &self.client_secret,
                    &self.redirect_uri,
                    "",
                    "",
                ));
            }
            _ => {}
        };
        self.client.as_ref().unwrap()
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
