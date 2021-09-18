use crate::{
    error::Error,
    message::{Data, Type},
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
        tokio::select! {
            message = default_handler::<Data<'_>>(&child[Privsep::PARENT_ID]) => {
                match message? {
                    (Message { id: Type::CONFIG, .. }, _, Data::Config(config)) => {
                        trace!("received config: {:?}", config);
                    }
                    (Message { id: Type::START, .. }, ..) => {
                        trace!("received start command");
                    }
                    _ => return Err(Error::InvalidMessage.into()),
                }
            }
            message = default_handler::<Data<'_>>(&child[Privsep::HEALTH_ID]) => {
                match message? {
                    (Message { id: Type::HOST_UP, .. }, _, Data::Host(id)) => {
                        trace!("received host UP: {}", id);
                    }
                    (Message { id: Type::HOST_DOWN, .. }, _, Data::Host(id)) => {
                        trace!("received host DOWN: {}", id);
                    }
                    _ => return Err(Error::InvalidMessage.into()),
                }
            }
        }
    }
}
