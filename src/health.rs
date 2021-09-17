use crate::{
    message::{self, Data},
    parent::default_handler,
    Child, Privsep,
};
use privsep::imsg::Message;
use privsep_log::{info, trace};
use std::sync::Arc;

pub async fn main<const N: usize>(
    child: Child<N>,
    config: privsep::Config,
) -> Result<(), privsep::Error> {
    let _guard = privsep_log::async_logger(&child.to_string(), &config)
        .await
        .map_err(|err| privsep::Error::GeneralError(Box::new(err)))?;

    let child = Arc::new(child);

    info!("Started");

    loop {
        let message = default_handler::<Data>(&child[Privsep::PARENT_ID]).await?;

        match message {
            Some((
                Message {
                    id: message::CONFIG,
                    ..
                },
                _,
                Data::Config(config),
            )) => {
                trace!("received config: {:?}", config);
            }
            None => {}
            _ => panic!("unexpected message"),
        }
    }
}
