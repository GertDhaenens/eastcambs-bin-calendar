use actix_web::{HttpResponse, Responder, Result, get};
use chrono::NaiveDate;
use dotenv;
use local_ip_address;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Mutex;
use uuid;

#[derive(Debug, Serialize, Deserialize)]
struct AppState {
    sequence: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CalendarQuery {
    urpn: u64,
    nocache: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
enum BagType {
    Black,
    Blue,
    GreenOrBrown,
}

#[get("/calendar")]
async fn get_collection_dates(
    app_state: actix_web::web::Data<Mutex<AppState>>,
    query: actix_web::web::Query<CalendarQuery>,
) -> Result<impl Responder> {
    println!("Fetching collection dates for urpn {0}...", query.urpn);

    let mut app_state_access = app_state.lock().unwrap();

    // Fetch the response from the web page
    let request_url = format!(
        "https://eastcambs-self.achieveservice.com/appshost/firmstep/self/apps/custompage/bincollections?uprn={0}",
        query.urpn
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

    // Create our calendar string
    let mut calendar_string = String::new();

    // Write our calendar header
    calendar_string.push_str("BEGIN:VCALENDAR\n");
    calendar_string.push_str("PRODID:-//Gert Dhaenens//East-Cambridgeshire Bin Collection//EN\n");
    calendar_string.push_str("VERSION:2.0\n");
    calendar_string.push_str("CALSCALE:Gregorian\n");
    calendar_string.push_str("METHOD:PUBLISH\n");
    calendar_string.push_str("X-WR-CALNAME:East-Cambridgeshire Bin Collection\n");
    calendar_string.push_str("X-WR-TIMEZONE:Europe/London\n");
    calendar_string.push_str("X-PUBLISHED-TTL:PT1W\n");

    // Fetch the current time for our DTTIMESTAMP values
    let time_now = chrono::offset::Utc::now();
    let time_now_str = time_now.format("%Y%m%dT%H%M%SZ").to_string();

    // Create a UUID namespace
    let calendar_namespace_uuid = uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_URL,
        "eastcambs-bin-collection".as_bytes(),
    );

    // Go over each collection & extract the info
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

        // Parse the date
        // For all day events - the DTSTART is the day, and the DTEND is the next day
        let start_date = NaiveDate::parse_from_str(date_str, "%a - %d %b %Y").unwrap();
        let end_date = start_date.checked_add_days(chrono::Days::new(1)).unwrap();

        // Generate our start and end date strings
        let start_date_string = start_date.format("%Y%m%dT000000").to_string();
        let end_date_string = end_date.format("%Y%m%dT000000").to_string();

        // Determine the name of our event based on the bag type
        let bag_type = match bag_str.trim().to_lowercase().as_str() {
            "black bag" => BagType::Black,
            "blue bin" => BagType::Blue,
            "green or brown bin" => BagType::GreenOrBrown,
            _ => panic!("TODO: Error response"),
        };
        let event_name = format!(
            "Bin collection - {}",
            match bag_type {
                BagType::Black => String::from("black bag(s)"),
                BagType::Blue => String::from("blue bin(s)"),
                BagType::GreenOrBrown => String::from("Green or brown bin(s)"),
            }
        );

        // Generate a unique UUID based on the date and name
        // This allows duplicate events to be matched in case of updates
        let event_uuid = uuid::Uuid::new_v5(&calendar_namespace_uuid, start_date_string.as_bytes());

        // Write our event string
        calendar_string.push_str("BEGIN:VEVENT\n");
        calendar_string.push_str(format!("SUMMARY:{}\n", event_name).as_str());
        calendar_string.push_str(format!("UID:{}\n", event_uuid.to_string()).as_str());
        calendar_string.push_str(format!("DTSTART;VALUE=DATE:{}\n", start_date_string).as_str());
        calendar_string.push_str(format!("DTEND;VALUE=DATE:{}\n", end_date_string).as_str());
        calendar_string.push_str(format!("DTSTAMP:{}\n", time_now_str).as_str());
        calendar_string.push_str(format!("LAST-MODIFIED:{}\n", time_now_str).as_str());
        calendar_string.push_str(format!("SEQUENCE:{}\n", app_state_access.sequence).as_str());
        calendar_string.push_str("TRANSP:TRANSPARENT\n");
        calendar_string.push_str("END:VEVENT\n");
    }

    // Write our calendar footer
    calendar_string.push_str("END:VCALENDAR");

    // Increment our sequence ID
    app_state_access.sequence += 1;

    Ok(HttpResponse::Ok()
        .content_type("text/calendar")
        .body(calendar_string))
}

#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    // Initialise the env logger
    unsafe { std::env::set_var("RUST_LOG", "debug") };
    env_logger::init();

    // Load out dotenv file to populate our env variables
    dotenv::dotenv().ok();

    // Fetch our local IP address to bind to
    let local_ip = local_ip_address::local_ip().expect("Failed to fetch local IP address");

    // Cache some state
    let app_state = actix_web::web::Data::new(Mutex::new(AppState { sequence: 0 }));

    // Create our web server
    println!("Starting web server...");
    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(app_state.clone())
            .service(get_collection_dates)
    })
    .bind(SocketAddr::new(local_ip, 8080))?
    .run()
    .await
}
