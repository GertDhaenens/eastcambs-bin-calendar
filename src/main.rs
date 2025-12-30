use anyhow::{Context, Result, bail};
use clap::Parser;
use reqwest;
use scraper;

#[derive(clap::Parser)]
struct CommandArgs {
    #[arg(short, long)]
    urpn: u64,
}

fn main() -> Result<()> {
    // Parse our arguments
    let args = CommandArgs::parse();

    // Fetch the response from the web page
    let request_url = format!(
        "https://eastcambs-self.achieveservice.com/appshost/firmstep/self/apps/custompage/bincollections?uprn={0}",
        args.urpn
    );
    let response = reqwest::blocking::get(request_url)
        .with_context(|| format!("Failed to request for urpn {0}", args.urpn))?
        .text()
        .unwrap();

    // There is no error reporting if the URPN is not valid, instead it returns a web page which
    // containts the address selection box again, so we search for this string in order to report
    // an error
    if response.contains("Please select an address to view the upcoming collections.") {
        bail!(
            "Failed to find collections for URPN {0}, please supply a valid URPN.",
            args.urpn
        );
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
    for collection_unparsed in collections_unparsed {
        let bag_str = collection_unparsed
            .select(&bag_selector)
            .next()
            .with_context(|| "Failed to parse bag div")?
            .text()
            .last()
            .with_context(|| "Failed to extract the text from the bag div")?;
        let date_str = collection_unparsed
            .select(&date_selector)
            .next()
            .with_context(|| "Failed to parse date div")?
            .text()
            .last()
            .with_context(|| "Failed to extract the text from the date div")?;

        // Determine the bag type
        println!("bag: {bag_str:?}");
        println!("date: {date_str:?}");
    }

    Ok(())
}
