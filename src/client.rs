use keyring;
use std::sync::Arc;

#[derive(Clone)]
pub struct Client {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    keyring_entry: Arc<keyring::Entry>,
    google_client: Option<google_calendar::Client>,
    google_access_token: Option<google_calendar::AccessToken>,
}

impl Client {
    pub fn new() -> Self {
        // Fetch the env vars
        Self {
            client_id: std::env::var("GOOGLE_OAUTH_CLIENT_ID")
                .expect("Expected \"GOOGLE_OAUTH_CLIENT_ID\" environment variable"),
            client_secret: std::env::var("GOOGLE_OAUTH_CLIENT_SECRET")
                .expect("Expected \"GOOGLE_OAUTH_CLIENT_SECRET\" environment variable"),
            redirect_uri: std::env::var("GOOGLE_OAUTH_REDIRECT_URI")
                .expect("Expected \"GOOGLE_OAUTH_REDIRECT_URI\" environment variable"),
            keyring_entry: Arc::new(
                keyring::Entry::new(
                    std::env::var("KEYRING_SERVICE_NAME")
                        .expect("Expected \"KEYRING_SERVICE_NAME\" environment variable")
                        .as_str(),
                    std::env::var("KEYRING_SERVICE_USER")
                        .expect("Expected \"KEYRING_SERVICE_USER\" environment variable")
                        .as_str(),
                )
                .unwrap(),
            ),
            google_client: None,
            google_access_token: None,
        }
    }

    pub async fn get_authenticate_url(&mut self) -> String {
        self.get_google_client()
            .await
            .user_consent_url(&["https://www.googleapis.com/auth/calendar.readonly".to_string()])
    }

    pub async fn authenticate(
        &mut self,
        code: &str,
        state: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self
            .get_google_client()
            .await
            .get_access_token(code, state)
            .await
        {
            Ok(t) => {
                // Store the refresh token to the keyring
                self.keyring_entry
                    .set_secret(t.refresh_token.as_bytes())
                    .expect("Failed to set keyring secret");
                // Store our token
                self.google_access_token = Some(t);
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    pub async fn has_authenticated(&mut self) -> bool {
        // Force our google client creation - which can refresh the access token
        self.get_google_client().await;
        self.google_access_token.is_some()
    }

    pub async fn get_all_calendars(
        &mut self,
    ) -> Option<Vec<google_calendar::types::CalendarListEntry>> {
        let google_client = self.get_google_client().await;
        let calendar_list = google_client.calendar_list();
        let calendars = calendar_list
            .list_all(google_calendar::types::MinAccessRole::Noop, false, false)
            .await;
        match calendars {
            Ok(v) => Some(v.body),
            Err(_) => None,
        }
    }

    async fn get_google_client(&mut self) -> &mut google_calendar::Client {
        match self.google_client {
            None => {
                // Get the secret from our keyring
                let refresh_token = match self.keyring_entry.get_secret() {
                    Ok(v) => {
                        println!("Got keyring secret refresh token");
                        String::from_utf8(v).unwrap()
                    }
                    Err(e) => {
                        println!("Failed to get keyring secret refresh token: {e:?}");
                        String::from("")
                    }
                };
                // Create our client - potentially using the existing refresh token
                self.google_client = Some(google_calendar::Client::new(
                    &self.client_id,
                    &self.client_secret,
                    &self.redirect_uri,
                    "",
                    refresh_token.clone(),
                ));
                // If we had a refresh token - refresh the access token
                if refresh_token.len() > 0 {
                    self.google_access_token = Some(
                        self.google_client
                            .as_mut()
                            .unwrap()
                            .refresh_access_token()
                            .await
                            .expect("Failed to refresh the access token"),
                    );
                }
            }
            _ => {}
        };
        self.google_client.as_mut().unwrap()
    }
}
