use actix_web::{Either, HttpResponse, Responder, Result, get, web::Redirect};

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
    client_state: actix_web::web::Data<ClientState>,
) -> actix_web::Result<impl Responder> {
    println!("Authenticating Google Calendar...");

    let user_state = client_state.user_state.lock().unwrap();
    if user_state.code.is_some() {
        println!("User already authenticated...");
        return Ok(Either::Left(HttpResponse::Ok().json(
            serde_json::json!({"status": "success", "message": "User already authorized"}),
        )));
    }

    // Create the client from the environment
    let client = google_calendar::Client::new(
        &client_state.client_id,
        &client_state.client_secret,
        &client_state.redirect_uri,
        "",
        "",
    );

    // Fetch the auth URL
    let user_consent_url =
        client.user_consent_url(&["https://www.googleapis.com/auth/calendar.readonly".to_string()]);

    // Redirect the user to the authentication
    Ok(Either::Right(Redirect::to(user_consent_url).temporary()))
}

#[get("/oauth/response")]
async fn authenticate_response(
    client_state: actix_web::web::Data<ClientState>,
    response: actix_web::web::Query<OAuthResponse>,
) -> Result<impl Responder> {
    // Verify that we have a code
    if response.code.is_empty() {
        return Ok(HttpResponse::Unauthorized().json(
            serde_json::json!({"status": "fail", "message": "Authorization code not provided!"}),
        ));
    }
    // Cache the state & code on the client state
    let mut user_state = client_state.user_state.lock().unwrap();
    user_state.state = Some(response.state.clone());
    user_state.code = Some(response.code.clone());
    Ok(HttpResponse::Ok()
        .json(serde_json::json!({"status": "success", "message": "User authorized"})))
}
