use actix_web::{Either, HttpResponse, Responder, Result, get, web::Redirect};

use crate::client::*;
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
    client: actix_web::web::Data<Mutex<Client>>,
) -> actix_web::Result<impl Responder> {
    println!("Authenticating Google Calendar...");

    // Fetch our client access
    let mut client_access = client.lock().unwrap();

    // If we already authenticated - early out
    if client_access.has_authenticated().await {
        println!("User already authenticated...");
        return Ok(Either::Left(HttpResponse::Unauthorized().json(
            serde_json::json!({"status": "success", "message": "User already authorised"}),
        )));
    }

    // Fetch the auth URL
    let user_consent_url = client_access.get_authenticate_url().await;

    // Redirect the user to the authentication
    println!("Redirecting user...");
    Ok(Either::Right(Redirect::to(user_consent_url).temporary()))
}

#[get("/oauth/response")]
async fn authenticate_response(
    client: actix_web::web::Data<Mutex<Client>>,
    response: actix_web::web::Query<OAuthResponse>,
) -> Result<impl Responder> {
    // Verify that we have a code
    if response.code.is_empty() {
        return Ok(HttpResponse::Unauthorized().json(
            serde_json::json!({"status": "fail", "message": "Authorization code not provided!"}),
        ));
    }

    // Fetch our access
    let mut client_access = client.lock().unwrap();

    // Get the access token using the code & state respone
    match client_access
        .authenticate(response.code.as_str(), response.state.as_str())
        .await
    {
        Ok(_) => {
            println!("User authenticated...");
            Ok(HttpResponse::Ok()
                .json(serde_json::json!({"status": "success", "message": "User authorized"})))
        }
        Err(_) => {
            println!("Authentication failed...");
            Ok(HttpResponse::Forbidden().json(
                serde_json::json!({"status": "failed", "message": "User authentication failed"}),
            ))
        }
    }
}
