use crate::Child;
use privsep_log::info;
use std::sync::Arc;

pub async fn main<const N: usize>(
    child: Child<N>,
    config: privsep::Config,
) -> Result<(), privsep::Error> {
    let _guard = privsep_log::async_logger(&child.to_string(), &config)
        .await
        .map_err(|err| privsep::Error::GeneralError(Box::new(err)))?;

    let _child = Arc::new(child);

    info!("Started");

    futures::future::pending().await
}
