use crate::{
    config::{Config, Variables},
    error::Error,
    message::{Data, Type},
    options::Options,
    Privsep,
};
use nix::sys::wait::{waitpid, WaitStatus};
use privsep::{
    imsg::Message,
    net::Fd,
    process::{daemon, Parent, Peer},
    Error as PrivsepError,
};
use privsep_log::{debug, info, warn};
use serde::de::DeserializeOwned;
use std::{io, process, sync::Arc};
use tokio::signal::unix::{signal, SignalKind};

pub async fn main<const N: usize>(
    parent: Parent<N>,
    privsep: privsep::Config,
) -> Result<(), privsep::Error> {
    let _guard = privsep_log::async_logger(&parent.to_string(), &privsep)
        .await
        .map_err(|err| PrivsepError::GeneralError(Box::new(err)))?;

    let parent = Arc::new(parent);

    let config = Config {
        privsep,
        ..init(&parent)
            .await
            .map_err(|err| PrivsepError::GeneralError(Box::new(err)))?
    };
    let mut sigchld = signal(SignalKind::child())?;

    // Detach the parent from the foreground.
    if !config.privsep.foreground {
        daemon(true, false)?;
    }

    info!("Started");

    // Send the configuration to all children.
    send_to_all(&parent, Type::Config, None, &Data::from(&config)).await?;
    send_to_all(&parent, Type::Start, None, &Data::None).await?;

    loop {
        tokio::select! {
            _ = sigchld.recv() => {
                match waitpid(None, None) {
                    Ok(WaitStatus::Exited(pid, status)) => {
                        warn!("Child {} exited with status {}", pid, status);
                        process::exit(0);
                    }
                    status => {
                        warn!("Child exited with error: {:?}", status);
                        process::exit(1);
                    }
                }
            }

            message = default_handler::<()>(&parent[Privsep::HEALTH_ID]) => { message?; },
            message = default_handler::<()>(&parent[Privsep::RELAY_ID]) => { message?; },
            message = default_handler::<()>(&parent[Privsep::REDIRECT_ID]) => { message?; },
        }
    }
}

async fn send_to_all<T: Into<Message> + Clone, const N: usize>(
    parent: &Parent<N>,
    id: T,
    fd: Option<&Fd>,
    data: &Data<'_>,
) -> io::Result<()> {
    for i in Privsep::PROCESS_IDS
        .iter()
        .filter(|i| **i != Privsep::PARENT_ID)
    {
        parent[*i].send_message(id.clone().into(), fd, data).await?;
    }

    Ok(())
}

pub async fn send_to_peer<T: Into<Message>>(
    peer: &Peer,
    id: T,
    fd: Option<&Fd>,
    data: &Data<'_>,
) -> io::Result<()> {
    peer.send_message(id.into(), fd, data).await
}

pub async fn init<const N: usize>(_parent: &Parent<N>) -> Result<Config, Error> {
    let opts = Options::new();
    let matches = opts.parse()?;

    let path = matches
        .opt_str("f")
        .unwrap_or_else(|| crate::RELAYD_CONFIG.to_string());

    let mut variables = Variables::new();
    for variable in matches.opt_strs("D") {
        let kv = variable.split('=').collect::<Vec<_>>();
        if kv.len() != 2 {
            return Err(Error::ParserError(variable));
        }
        variables.insert(kv[0].to_string(), kv[1].to_string());
    }

    let config = Config::load(&path, variables).await?;

    Ok(config)
}

pub async fn default_handler<T: DeserializeOwned>(
    peer: &Peer,
) -> Result<(Message, Option<Fd>, T), Error> {
    match peer.recv_message::<T>().await? {
        None => Err(Error::Terminated(peer.as_ref())),
        Some((message, fd, data)) => {
            debug!(
                "received message {}", Type::from(message.id);
                "source" => peer.as_ref(),
            );

            Ok((message, fd, data))
        }
    }
}
