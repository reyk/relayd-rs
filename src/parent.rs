use nix::sys::wait::{waitpid, WaitStatus};
use privsep::process::Parent;
use privsep_log::{info, warn};
use std::{process, sync::Arc};
use tokio::signal::unix::{signal, SignalKind};

pub async fn main<const N: usize>(
    parent: Parent<N>,
    config: privsep::Config,
) -> Result<(), privsep::Error> {
    let _guard = privsep_log::async_logger(&parent.to_string(), &config)
        .await
        .map_err(|err| privsep::Error::GeneralError(Box::new(err)))?;

    let _parent = Arc::new(parent);

    info!("Started");

    let mut sigchld = signal(SignalKind::child())?;

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
