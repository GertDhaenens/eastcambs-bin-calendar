use actix_web::{HttpResponse, Responder, Result, post};

use crate::client::*;
use crate::types::*;

#[derive(Deserialize)]
struct SetupForm {
    urpn: u64,
    calendar_id: String,
}

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(actix_files::Files::new("/html", "html").show_files_listing())
        .route("/setup", actix_web::web::get().to(setup_get))
        .service(setup_post);
}

async fn setup_get(client: actix_web::web::Data<Mutex<Client>>) -> Result<impl Responder> {
    println!("Configuring setup...");

    // Fetch the calendars from the client
    let mut client_access = client.lock().unwrap();
    let calendar_entries = client_access.get_all_calendars().await.unwrap_or(vec![]);

    let values = serde_json::json!({
        "urpn": match client_access.get_urpn() {
            Some(v) => v,
            None => 0
        },
        "calendars": calendar_entries.into_iter().map(|calendar| serde_json::json!({
            "id": calendar.id,
            "name": calendar.summary,
            "selected": match client_access.get_calendar_id() {
                Some(id) => id == calendar.id,
                None => false
            }
        })
        ).collect::<Vec<_>>()
    });
    println!("{values:?}");
    let hb = &handlebars_ref;
    let body = hb.render("setup", &values).unwrap();
    Ok(HttpResponse::Ok().body(body))
}

#[post("/setup")]
async fn setup_post(
    client: actix_web::web::Data<Mutex<Client>>,
    data: actix_web::web::Form<SetupForm>,
) -> Result<impl Responder> {
    println!("We got a setup response...");
    println!("URPN: {0}", data.urpn);
    println!("Calendar ID: {0}", data.calendar_id);
    let mut client_access = client.lock().unwrap();
    client_access.set_urpn(data.urpn);
    client_access.set_calendar_id(data.calendar_id.clone());

    Ok(HttpResponse::Ok().body("Hello"))
}
