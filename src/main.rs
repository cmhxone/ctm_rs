use std::{error::Error, thread, time::Duration};

use ctm::cti_client::CTIClient;

mod cisco;
mod ctm;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log4rs.yml", Default::default())?;

    let cti_client = CTIClient::new(true).await?;
    cti_client.connect().await?;

    loop {
        thread::sleep(Duration::from_millis(1000));
    }

    #[allow(unreachable_code)]
    Ok(())
}
