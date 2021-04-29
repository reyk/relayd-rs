use relayd::{Options, Privsep};
use std::process;

#[tokio::main]
async fn main() {
    let opts = Options::new();

    let matches = match opts.parse() {
        Ok(matches) => matches,
        Err(_) => process::exit(1),
    };

    let config = privsep::Config {
        foreground: matches.opt_present("d"),
        log_level: privsep_log::verbose(matches.opt_count("v")).into(),
    };

    if let Err(err) = Privsep::main(config).await {
        eprintln!("{}: {}", opts, err);
        process::exit(1);
    }
}
