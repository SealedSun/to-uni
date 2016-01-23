
use ::common::*;
use ::config::Configuration;

use ::aho_corasick::AcAutomaton;

/// Performs substitution on a single input stream according to the supplied configuration.
pub fn run(config: &Configuration) -> UniResult<()> {

    let automaton = AcAutomaton::new(config.patterns.keys());

    let mut output = try!(config.output.open());
    {
        // Region where the input file is open
        let mut input = try!(config.input.open());

        let mut bytes_written = 0u64;
        for m in automaton.stream_find(input) {
            
        }
    }
    

    Ok(())
}
