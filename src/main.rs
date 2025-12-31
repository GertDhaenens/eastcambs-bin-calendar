use dotenv;

mod dates;
mod google_oauth;
mod types;

use crate::types::*;

#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    // Initialise the env logger
    unsafe { std::env::set_var("RUST_LOG", "debug") };
    env_logger::init();

    // Load out dotenv file to populate our env variables
    dotenv::dotenv().ok();

    // Fetch our client info from env vars *before* we kick off the server
    let app_state = actix_web::web::Data::new(Mutex::new(AppState::new(
        std::env::var("GOOGLE_OAUTH_CLIENT_ID")
            .expect("Expected \"GOOGLE_OAUTH_CLIENT_ID\" environment variable"),
        std::env::var("GOOGLE_OAUTH_CLIENT_SECRET")
            .expect("Expected \"GOOGLE_OAUTH_CLIENT_SECRET\" environment variable"),
        std::env::var("GOOGLE_OAUTH_REDIRECT_URI")
            .expect("Expected \"GOOGLE_OAUTH_REDIRECT_URI\" environment variable"),
    )));

    // Create our web server
    println!("Starting web server...");
    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(app_state.clone())
            .configure(google_oauth::config)
            .configure(dates::config)
    })
    .bind("localhost:8080")?
    .run()
    .await
}
