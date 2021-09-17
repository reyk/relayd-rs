use relayd::{Options, Privsep};
use std::{env, process};

#[tokio::main]
async fn main() {
    let opts = Options::new();

    let matches = match opts.parse() {
        Ok(matches) => matches,
        Err(_) => process::exit(1),
    };

    let log_level = env::var("RUST_LOG")
        .unwrap_or_else(|_| privsep_log::verbose(matches.opt_count("v")))
        .into();

    let config = privsep::Config {
        foreground: matches.opt_present("d"),
        log_level,
    };

    if let Err(err) = Privsep::main(config).await {
        eprintln!("{}: {}", opts, err);
        process::exit(1);
    }
}
