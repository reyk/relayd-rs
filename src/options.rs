use crate::error::Error;
use derive_more::{Deref, DerefMut, Display};
use std::env;

#[derive(Deref, DerefMut, Display)]
#[display(fmt = "{}", prog)]
pub struct Options {
    #[deref]
    #[deref_mut]
    opts: getopts::Options,
    pub args: Vec<String>,
    pub prog: String,
}

impl Options {
    pub fn new() -> Self {
        let mut args: Vec<String> = env::args().collect();
        let prog = args.remove(0);

        let mut opts = getopts::Options::new();

        opts.optflag("d", "", "Do not daemonize");
        opts.optflagmulti("v", "", "Enable verbose logging");

        Self { args, prog, opts }
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
