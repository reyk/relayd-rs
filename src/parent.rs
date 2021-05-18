use crate::{
    config::{Config, Variables},
    error::Error,
    options::Options,
};
use nix::sys::wait::{waitpid, WaitStatus};
use privsep::{process::Parent, Error as PrivsepError};
use privsep_log::{info, warn};
use std::{process, sync::Arc};
use tokio::signal::unix::{signal, SignalKind};

pub async fn main<const N: usize>(
    parent: Parent<N>,
    log_config: privsep::Config,
) -> Result<(), privsep::Error> {
    let _guard = privsep_log::async_logger(&parent.to_string(), &log_config)
        .await
        .map_err(|err| PrivsepError::GeneralError(Box::new(err)))?;

    init(parent)
        .await
        .map_err(|err| PrivsepError::GeneralError(Box::new(err)))?;
    let mut sigchld = signal(SignalKind::child())?;

    info!("Started");

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
        }
    }
}

pub async fn init<const N: usize>(parent: Parent<N>) -> Result<(), Error> {
    let _parent = Arc::new(parent);

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

    let _config = Config::load(&path, variables).await?;

    Ok(())
}
