use std::error::Error;

use ctm::CTM;

mod cisco;
mod ctm;
mod event;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log4rs.yml", Default::default())?;

    let ctm = CTM::new().await?;
    ctm.start().await?;

    #[allow(unreachable_code)]
    Ok(())
}
