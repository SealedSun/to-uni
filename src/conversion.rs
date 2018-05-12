
extern crate stopwatch;

use ::common::*;
use ::config::Configuration;
use ::error::{UniError,code, UniErrorData};

use ::aho_corasick::{AcAutomaton,Automaton};
use ::aho_corasick::chunked::{StreamChunks,StreamChunk,StreamChunkError};

use self::stopwatch::Stopwatch;

/// Performs substitution on a single input stream according to the supplied configuration.
pub fn run(config: &Configuration) -> UniResult<()> {

    debug!("Configured input: {:#?}", config.input);
    debug!("Configured output: {:#?}", config.output);

    info!("Computing matching automaton ({} patterns)...", config.patterns.len());
    let stopwatch = Stopwatch::start_new();
    let automaton = AcAutomaton::new(config.patterns.keys().map(|p| format!("\\{}", p)));
    let lookup_map : Vec<&str> = automaton.patterns().iter().map::<&str,_>(|p| &config.patterns[&p[1 ..]] ).collect();
    info!("Matching automaton for {} patterns computed in {}ms", config.patterns.len(), stopwatch.elapsed_ms());


    let mut output = config.output.open()?;
    {
        // Region where the input file is open
        let input = config.input.open()?;
        let mut chunks = StreamChunks::with_capacity(&automaton, input, 512);
        chunks.all::<_, UniError>(|chunk| {
            let out_bytes = match chunk {
                StreamChunk::Matching(m) => {
                    // TODO: skip text-based lookup in favour of pattern index.
                    let replacement = lookup_map[m.pati];
                    debug!("Found {} replacing it with {}", automaton.pattern(m.pati), replacement);
                    replacement.as_bytes()
                },
                StreamChunk::NonMatching(bs) => {
                    debug!("Forwarding {} non-matching bytes.", bs.len());
                    bs
                }
            };
            match output.write_all(out_bytes) {
                Err(ioe) => Err(UniError::new(code::fsio::OUTPUT, UniErrorData::Io(ioe))),
                Ok(()) => Ok(())
            }
        })?;
    }

    // Return the output writer; behaviour depends on what the user asked for
    config.output.close(output)
}

// This automatic conversion affects the input stream. Output IO errors are handled explicitly.
impl From<StreamChunkError<UniError>> for UniError {
    fn from(e: StreamChunkError<UniError>) -> UniError {
        match e {
            StreamChunkError::User(ue) => ue,
            StreamChunkError::Io(ioe) => UniError::new(code::fsio::INPUT, UniErrorData::Io(ioe))
        }
    }
}
