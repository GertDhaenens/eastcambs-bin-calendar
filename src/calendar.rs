use actix_web::{HttpResponse, Responder, Result, get};
use serde_json;

use crate::{client::*, types::*};

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(list_calendars);
}

#[get("/calendars")]
async fn list_calendars(client: actix_web::web::Data<Mutex<Client>>) -> Result<impl Responder> {
    // Fetch our app state
    let mut client_access = client.lock().unwrap();

    // Request all calendars
    let calendars = client_access.get_all_calendars().await;
    match calendars {
        Some(c) => Ok(HttpResponse::Ok().json(
            serde_json::json!({ "status": "success", "message": "Calendars listed...", "data": c }),
        )),
        None => Ok(HttpResponse::BadRequest().json(
            serde_json::json!({ "status": "failed", "message": "Failed to list all calendars" }),
        )),
    }
}
