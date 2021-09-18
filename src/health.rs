use crate::{
    error::Error,
    message::{Data, Type},
    parent::{default_handler, send_to_peer},
    Child, Context, Privsep,
};
use futures::{stream::FuturesUnordered, StreamExt};
use privsep::imsg::Message;
use privsep_log::{debug, info, trace};
use std::{io, sync::Arc};
use tokio::{net, time};

pub async fn main<const N: usize>(
    child: Child<N>,
    privsep_config: privsep::Config,
) -> Result<(), privsep::Error> {
    let _guard = privsep_log::async_logger(&child.to_string(), &privsep_config)
        .await
        .map_err(|err| privsep::Error::GeneralError(Box::new(err)))?;

    let context = Context {
        child: Arc::new(child),
        config: Default::default(),
    };

    info!("Started");

    loop {
        tokio::select! {
            message = default_handler::<Data<'_>>(&context.child[Privsep::PARENT_ID]) => {
                match message? {
                    (Message { id: Type::CONFIG, .. }, _, Data::Config(new_config)) => {
                        trace!("received config: {:?}", new_config);
                        context.config.store(Arc::new(new_config.into_owned()));
                    }
                    (Message { id: Type::START, .. }, ..) => {
                        trace!("received start command");
                        run(context.clone()).await
                    }
                    _ => return Err(Error::InvalidMessage.into()),
                }
            }
            message = default_handler::<()>(&context.child[Privsep::RELAY_ID]) => { message?; },
            message = default_handler::<()>(&context.child[Privsep::REDIRECT_ID]) => { message?; },
        }
    }
}

async fn run<const N: usize>(context: Context<N>) {
    trace!("Running");

    tokio::spawn(async move {
        let mut interval = time::interval(context.config.load().interval);
        loop {
            interval.tick().await;
            debug!("tick");

            let context = context.clone();
            let config = context.config.load();

            tokio::spawn(async move {
                let mut tasks = FuturesUnordered::new();
                let timeout = config.timeout;

                for host in config
                    .tables
                    .iter()
                    .map(|table| table.hosts.iter())
                    .flatten()
                {
                    let host = host.clone();
                    let id = host.id;
                    let fut = tokio::spawn(async move {
                        let addr = host.name + ":80";
                        let addrs = net::lookup_host(&addr).await.map_err(|_| id)?;
                        for addr in addrs {
                            debug!("checking host {}: {}", id, addr);
                            if let Ok(Ok(_)) =
                                time::timeout(timeout, net::TcpStream::connect(addr)).await
                            {
                                return Ok(id);
                            }
                        }
                        Err(id)
                    });
                    tasks.push(fut);
                }

                while let Some(result) = tasks.next().await {
                    let (typ, id) = match result? {
                        Ok(id) => {
                            debug!("host {} is UP", id);
                            (Type::HostUp, id)
                        }
                        Err(id) => {
                            debug!("host {} is DOWN", id);
                            (Type::HostDown, id)
                        }
                    };

                    let peer = &context.child[Privsep::REDIRECT_ID];
                    send_to_peer(peer, typ, None, &Data::Host(id))
                        .await
                        .unwrap();
                    let peer = &context.child[Privsep::RELAY_ID];
                    send_to_peer(peer, typ, None, &Data::Host(id))
                        .await
                        .unwrap();
                }
                Ok::<_, io::Error>(())
            });
        }
    });
}
