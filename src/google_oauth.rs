use actix_web::{HttpResponse, Responder, Result, get, web::Redirect};

use crate::types::*;

#[derive(Debug, Serialize, Deserialize)]
struct OAuthResponse {
    state: String,
    code: String,
}

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(authenticate).service(authenticate_response);
}

#[get("/oauth")]
async fn authenticate(
    app_state: actix_web::web::Data<Mutex<AppState>>,
) -> actix_web::Result<impl Responder> {
    println!("Authenticating Google Calendar...");

    // Fetch our google client
    let mut app_state_access = app_state.lock().unwrap();
    let google_client = app_state_access.get_client();

    // Fetch the auth URL
    let user_consent_url = google_client
        .user_consent_url(&["https://www.googleapis.com/auth/calendar.readonly".to_string()]);

    // Redirect the user to the authentication
    Ok(Redirect::to(user_consent_url).temporary())
}

#[get("/oauth/response")]
async fn authenticate_response(
    _app_state: actix_web::web::Data<Mutex<AppState>>,
    response: actix_web::web::Query<OAuthResponse>,
) -> Result<impl Responder> {
    // Verify that we have a code
    if response.code.is_empty() {
        return Ok(HttpResponse::Unauthorized().json(
            serde_json::json!({"status": "fail", "message": "Authorization code not provided!"}),
        ));
    }

    println!("User authenticated...");
    Ok(HttpResponse::Ok()
        .json(serde_json::json!({"status": "success", "message": "User authorized"})))
}
