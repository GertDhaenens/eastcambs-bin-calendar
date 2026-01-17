use actix_web::{HttpResponse, Responder, Result, get};
use bitflags::bitflags;
use chrono::NaiveDate;
use dotenv;
use local_ip_address;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
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

bitflags! {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct BagType: u8 {
        const NONE = 0;
        const BLACK = 1 << 0;
        const BLUE = 1 << 1;
        const GREEN_OR_BROWN = 1 << 2;
    }
}

impl BagType {
    pub fn to_string(&self) -> String {
        let mut strings = Vec::new();
        if self.intersects(BagType::BLACK) {
            strings.push("black");
        }
        if self.intersects(BagType::BLUE) {
            strings.push("blue");
        }
        if self.intersects(BagType::GREEN_OR_BROWN) {
            strings.push("green/brown");
        }
        if strings.is_empty() {
            String::from("none")
        } else if strings.len() == 1 {
            String::from(strings[0])
        } else if strings.len() == 2 {
            format!("{} & {}", strings[0], strings[1])
        } else {
            let (last, other) = strings.split_last().unwrap();
            format!("{} & {}", other.join(", "), last)
        }
    }
}

struct Collection {
    date: NaiveDate,
    bag_types: BagType,
}

async fn get_collection_dates(urpn: u64) -> Result<Vec<Collection>> {
    println!("Fetching collection dates for urpn {0}...", urpn);

    // Fetch the response from the web page
    let request_url = format!(
        "https://eastcambs-self.achieveservice.com/appshost/firmstep/self/apps/custompage/bincollections?uprn={0}",
        urpn
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
    let mut collections_by_date = HashMap::new();
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
        let collection_date = NaiveDate::parse_from_str(date_str, "%a - %d %b %Y").unwrap();

        // Parse the bag types
        let bag_types = match bag_str.trim().to_lowercase().as_str() {
            "black bag" => BagType::BLACK,
            "blue bin" => BagType::BLUE,
            "green or brown bin" => BagType::GREEN_OR_BROWN,
            _ => panic!("TODO: Error response"),
        };

        // We can have multiple collections on the same day - so combine them
        if collections_by_date.contains_key(&collection_date) {
            let collection: &mut Collection =
                collections_by_date.get_mut(&collection_date).unwrap();
            collection.bag_types |= bag_types;
        } else {
            collections_by_date.insert(
                collection_date,
                Collection {
                    date: collection_date,
                    bag_types: bag_types,
                },
            );
        }
    }

    // Build our vector by extracting the elements from our hash map
    // now that they have been combined, and sort them based on date
    let mut collections = collections_by_date.into_values().collect::<Vec<_>>();
    collections.sort_by(|a, b| a.date.cmp(&b.date));

    Ok(collections)
}

#[get("/calendar")]
async fn get_ics_file(
    app_state: actix_web::web::Data<Mutex<AppState>>,
    query: actix_web::web::Query<CalendarQuery>,
) -> Result<impl Responder> {
    let mut app_state_access = app_state.lock().unwrap();

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

    // Fetch the collection info
    let collections = get_collection_dates(query.urpn).await?;

    // Go over each collection & add an event to the calendar
    for collection in collections {
        // For all day events - the DTSTART is the day, and the DTEND is the next day
        let start_date = collection.date;
        let end_date = start_date.checked_add_days(chrono::Days::new(1)).unwrap();

        // Generate our start and end date strings
        let start_date_string = start_date.format("%Y%m%dT000000").to_string();
        let end_date_string = end_date.format("%Y%m%dT000000").to_string();

        let event_name = format!("Bin collection - {}", collection.bag_types.to_string());

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

#[get("/trmnl")]
async fn get_trmnl_json(
    _app_state: actix_web::web::Data<Mutex<AppState>>,
    query: actix_web::web::Query<CalendarQuery>,
) -> Result<impl Responder> {
    // Fetch the dates
    let collections = get_collection_dates(query.urpn).await?;

    // We only care about the most recent one
    let collection = &collections[0];

    // Build some nicely formatted strings
    let type_str = collection.bag_types.to_string();

    // Format our date
    let date_str = collection.date.format("%a, %d %h").to_string();

    // Calculate the time until the collection
    let time_now_utc = chrono::offset::Utc::now();
    let date_now = time_now_utc.date_naive();
    let time_diff = NaiveDate::signed_duration_since(collection.date, date_now);
    let days_until = time_diff.num_days();
    let time_until_str = if days_until == 1 {
        String::from("1 day")
    } else {
        format!("{} days", days_until)
    };

    Ok(actix_web::web::Json(json!({
        "type": type_str,
        "date": date_str,
        "time_until": time_until_str
    })))
}

#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    // Debug-only features
    if cfg!(debug_assertions) {
        // Initialise the env logger
        unsafe { std::env::set_var("RUST_LOG", "debug") };
        env_logger::init();

        // Load out dotenv file to populate our env variables
        dotenv::dotenv().ok();
    }

    // Fetch our local IP address to bind to
    let local_ip = local_ip_address::local_ip().expect("Failed to fetch local IP address");

    // Cache some state
    let app_state = actix_web::web::Data::new(Mutex::new(AppState { sequence: 0 }));

    // Create our web server
    println!("Starting web server...");
    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(app_state.clone())
            .service(get_ics_file)
            .service(get_trmnl_json)
    })
    .bind(SocketAddr::new(local_ip, 8080))?
    .run()
    .await
}
