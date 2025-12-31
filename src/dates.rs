use actix_web::{HttpResponse, Responder, Result, get};
use reqwest;
use scraper;

use crate::client::*;
use crate::types::*;

#[derive(Debug, Serialize, Deserialize)]
struct DatesQuery {
    urpn: u64,
}

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(gather_dates);
}

#[get("/dates")]
async fn gather_dates(
    _client: actix_web::web::Data<Mutex<Client>>,
    query: actix_web::web::Query<DatesQuery>,
) -> Result<impl Responder> {
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
