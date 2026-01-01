use actix_web::{HttpResponse, Responder, Result, get};
use chrono::NaiveDate;
use dotenv;
use ics;
use local_ip_address;
use serde::{Deserialize, Serialize};
use std::{fmt::Write, net::SocketAddr};
use uuid;

#[derive(Debug, Serialize, Deserialize)]
enum BagType {
    Black,
    Blue,
    GreenOrBrown,
}

#[derive(Debug, Serialize, Deserialize)]
struct DatesQuery {
    urpn: u64,
}

#[get("/dates")]
async fn get_collection_dates(query: actix_web::web::Query<DatesQuery>) -> Result<impl Responder> {
    println!("Fetching collection dates for urpn {0}...", query.urpn);

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

    // Create our calendar object
    let mut calendar = ics::ICalendar::new("2.0", "ics-rs");

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

        // Parse the bag type
        let bag_type = match bag_str.trim().to_lowercase().as_str() {
            "black bag" => BagType::Black,
            "blue bin" => BagType::Blue,
            "green or brown bin" => BagType::GreenOrBrown,
            _ => panic!("TODO: Error response"),
        };

        // Parse the date
        let date = NaiveDate::parse_from_str(date_str, "%a - %d %b %Y").unwrap();

        // Add our event to the calendar
        calendar.add_event(ics::Event::new(
            uuid::Uuid::new_v4().to_string(),
            date.to_string(),
        ));

        // DEBUG ONLY: Only write first event while we test this
        break;
    }

    // Write our calendar to a string
    let mut calendar_string = String::new();
    write!(&mut calendar_string, "{}", calendar).expect("Failed to write calendar to string");

    println!("{calendar_string}");

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

    // Create our web server
    println!("Starting web server...");
    actix_web::HttpServer::new(move || actix_web::App::new().service(get_collection_dates))
        .bind(SocketAddr::new(local_ip, 8080))?
        .run()
        .await
}
