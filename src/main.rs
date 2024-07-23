use lambda_http::Error;
use nxtpoll_api::{config::Config, run_app};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = Config::from_env();
    run_app(config).await
}
