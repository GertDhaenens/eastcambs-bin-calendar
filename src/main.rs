use actix_web::{Either, HttpResponse, Responder, Result, get, web::Redirect};
use chrono::NaiveDate;
use clap::Parser;
use google_calendar;
use reqwest;
use scraper;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::{Arc, Mutex};

#[derive(clap::Parser)]
struct CommandArgs {
    #[arg(short, long)]
    urpn: u64,
}

struct UserState {
    state: Option<String>,
    code: Option<String>,
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
struct ClientState {
    urpn: u64,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    user_state: Arc<Mutex<UserState>>,
}

impl ClientState {
    pub fn new(urpn: u64, client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            urpn: urpn,
            client_id: client_id,
            client_secret: client_secret,
            redirect_uri: redirect_uri,
            user_state: Arc::new(Mutex::new(UserState::new())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OAuthResponse {
    state: String,
    code: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum BagType {
    Black,
    Blue,
    GreenOrBrown,
}

#[derive(Debug, Serialize, Deserialize)]
struct CollectionDate {
    bag_type: BagType,
    date: NaiveDate,
}

#[get("/oauth")]
async fn authenticate_google_calendar(
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
async fn authenticate_google_calendar_response(
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

#[get("/dates")]
async fn fetch_collection_dates(
    client_state: actix_web::web::Data<ClientState>,
) -> Result<impl Responder> {
    println!("Fetching collection dates...");

    // Fetch the response from the web page
    let request_url = format!(
        "https://eastcambs-self.achieveservice.com/appshost/firmstep/self/apps/custompage/bincollections?uprn={0}",
        client_state.urpn
    );
    let response = reqwest::get(request_url)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // There is no error reporting if the URPN is not valid, instead it returns a web page which
    // containts the address selection box again, so we search for this string in order to report
    // an error
    if response.contains("Please select an address to view the upcoming collections.") {
        panic!("TODO: Error response")
    }

    // Parse the HTML that was as response
    let parsed_html = scraper::Html::parse_document(response.as_str());

    // Find all of the collection divs
    let selector = scraper::Selector::parse("div.row.collectionsrow:not(.panel-collapse)").unwrap();
    let collections_unparsed = parsed_html.select(&selector);

    // Create our selectors for the child elements
    let bag_selector = scraper::Selector::parse("div.col-xs-4.col-sm-4").unwrap();
    let date_selector = scraper::Selector::parse("div.col-xs-6.col-sm-6").unwrap();

    // Go over each collection & extract the info
    let mut collection_dates = Vec::new();
    for collection_unparsed in collections_unparsed {
        let bag_str = collection_unparsed
            .select(&bag_selector)
            .next()
            .unwrap()
            .text()
            .last()
            .unwrap();
        let date_str = collection_unparsed
            .select(&date_selector)
            .next()
            .unwrap()
            .text()
            .last()
            .unwrap();

        // Add the collection date
        collection_dates.push(CollectionDate {
            bag_type: match bag_str.trim().to_lowercase().as_str() {
                "black bag" => BagType::Black,
                "blue bin" => BagType::Blue,
                "green or brown bin" => BagType::GreenOrBrown,
                _ => panic!("TODO: Error response"),
            },
            date: NaiveDate::parse_from_str(date_str, "%a - %d %b %Y").unwrap(),
        });
    }

    Ok(HttpResponse::Ok().json(serde_json::to_string(&collection_dates).unwrap()))
}

#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    // Initialise the env logger
    unsafe { std::env::set_var("RUST_LOG", "debug") };
    env_logger::init();

    // Parse our arguments
    let args = CommandArgs::parse();

    // Fetch our client info from env vars *before* we kick off the server
    let client_state = actix_web::web::Data::new(ClientState::new(
        args.urpn,
        std::env::var("GOOGLE_CALENDAR_CLIENT_ID")
            .expect("Expected \"GOOGLE_CALENDAR_CLIENT_ID\" environment variable"),
        std::env::var("GOOGLE_CALENDAR_CLIENT_SECRET")
            .expect("Expected \"GOOGLE_CALENDAR_CLIENT_SECRET\" environment variable"),
        std::env::var("GOOGLE_CALENDAR_REDIRECT_URI")
            .expect("Expected \"GOOGLE_CALENDAR_REDIRECT_URI\" environment variable"),
    ));
    println!("URPN: {0}", client_state.urpn);
    println!("Client ID: {0}", client_state.client_id);
    println!("Client Secret: {0}", client_state.client_secret);
    println!("Redirect URI: {0}", client_state.redirect_uri);

    // Create our web server
    println!("Starting server...");
    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(client_state.clone())
            .service(authenticate_google_calendar)
            .service(authenticate_google_calendar_response)
            .service(fetch_collection_dates)
    })
    .bind("localhost:8080")?
    .run()
    .await
}
