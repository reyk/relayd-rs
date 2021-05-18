use crate::error::Error;
use derive_more::{Deref, DerefMut, Display};
use std::env;

#[derive(Deref, DerefMut, Display)]
#[display(fmt = "{}", prog)]
pub struct Options {
    pub args: Vec<String>,
    #[deref]
    #[deref_mut]
    opts: getopts::Options,
    pub prog: String,
}

impl Options {
    pub fn new() -> Self {
        let mut args: Vec<String> = env::args().collect();
        let prog = args.remove(0);

        let mut opts = getopts::Options::new();

        opts.optflag("d", "", "Do not daemonize");
        opts.optopt(
            "f",
            "",
            "Specify an alternative configuration file",
            crate::RELAYD_CONFIG,
        );
        opts.optmulti(
            "D",
            "",
            "Define macro to be set to value on the command line",
            "macro=value",
        );
        opts.optflagmulti("v", "verbose", "Enable verbose logging");

        Self { args, opts, prog }
    }

    pub fn parse(&self) -> Result<getopts::Matches, Error> {
        match self.opts.parse(&self.args) {
            Ok(matches) => Ok(matches),
            Err(err) => {
                eprintln!("{}: {}", self, err);
                self.usage();
                Err(err.into())
            }
        }
    }

    pub fn usage(&self) {
        eprintln!("{}", self.opts.short_usage(&self.prog));
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}
