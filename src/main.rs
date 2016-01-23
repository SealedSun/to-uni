
#[macro_use]
extern crate log;
extern crate docopt;
extern crate rustc_serialize;
extern crate env_logger;
extern crate yaml_rust as yaml;
extern crate atomicwrites;
extern crate aho_corasick;

mod common;
#[macro_use]
mod error;
mod config;
mod conversion;

use docopt::Docopt;

fn main() {
    common::init();
    // the docopt::Error::exit method automatically prints help (and version) as appropriate
    let args: config::Args = Docopt::new(config::USAGE).and_then(|d| 
          d.help(true)
              .version(Some(String::from(common::TO_UNI_VERSION)))
              .decode())
        .unwrap_or_else(|e| e.exit());
    debug!("Command line arguments: {:#?}", args);

    common::handle_program_exit(
        config::Configuration::from_args(args).and_then(|c| conversion::run(&c))
    );
}


