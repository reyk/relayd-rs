use getopts::Options;
use relayd::Privsep;
use std::{env, process};

fn usage(program: &str, opts: Options) -> ! {
    eprintln!("{}", opts.short_usage(program));
    process::exit(1);
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("d", "", "Do not daemonize");
    opts.optflagmulti("v", "", "Enable verbose logging");
    let matches = match opts.parse(&args[1..]) {
        Ok(matches) => matches,
        Err(err) => {
            eprintln!("{}", err);
            usage(&program, opts);
        }
    };

    let log_level = privsep_log::verbose(matches.opt_count("v")).into();

    let config = privsep::Config {
        foreground: matches.opt_present("d"),
        log_level,
    };

    if let Err(err) = Privsep::main(config).await {
        eprintln!("{}", err);
        process::exit(1);
    }
}
