use lambda_http::Error;
use nxtpoll_api::{run_app, config::Config};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = Config::from_env();
    run_app(config).await
}
