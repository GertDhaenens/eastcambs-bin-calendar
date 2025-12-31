use dotenv;

mod calendar;
mod client;
mod dates;
mod google_oauth;
mod setup;
mod types;

use crate::client::*;
use crate::types::*;

#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    // Initialise the env logger
    unsafe { std::env::set_var("RUST_LOG", "debug") };
    env_logger::init();

    // Load out dotenv file to populate our env variables
    dotenv::dotenv().ok();

    // Initialise handlebars

    // Fetch our client info from env vars *before* we kick off the server
    let client = actix_web::web::Data::new(Mutex::new(Client::new()));

    // Create our web server
    println!("Starting web server...");
    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(client.clone())
            .configure(google_oauth::config)
            .configure(dates::config)
            .configure(calendar::config)
            .configure(setup::config)
    })
    .bind("localhost:8080")?
    .run()
    .await
}
