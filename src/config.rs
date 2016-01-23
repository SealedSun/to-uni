
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::io::{self,Read,Write, stdin, stdout};
use std::fs::{self, File};
use std::env;

use ::yaml::Yaml;

use ::common::*;
use ::error;

pub static USAGE: &'static str ="
to-uni is a program that scans for LaTeX-style escape sequences in its input and replaces 
them with their unicode counterpart.

Usage:
    to-uni [options] (<input>|[-]) [<output>|--stdout]
    to-uni --version
    to-uni -h | --help

Options:
    -h --help                   Show this screen
    --version                   Show the version and exit
    --stdout                    Write converted stream to standard output
    --no-backup -B              When doing an in-place conversion, don't create a backup of 
                                the original
    --config=CONFIG             Specific configuration file or search origin. 
                                By default, to-uni uses
                                the directory of the input file as a starting point and searches 
                                upwards
                                in the file hierarchy until CFGNAME is found.
    --config-name=CFGNAME       Name of the to-uni configuration file (YAML) [default: to-uni.yml]

";

#[derive(Debug,RustcDecodable)]
#[allow(non_snake_case)]
pub struct Args {
    arg_input: Option<String>,
    arg_output: Option<String>,
    flag_config: Option<String>,
    flag_config_name: String,
    flag_stdout: bool,
    flag_no_backup: bool
}

pub enum Input {
    /// Source file
    File(PathBuf),
    /// Stdin
    Stdin
}

impl Input {
    pub fn directory(&self) -> UniResult<PathBuf> {
        match *self {
            Input::Stdin => Ok(try!(env::current_dir())),
            Input::File(ref buf) => {
                let base = try_!(buf.parent().ok_or("File does not have a parent directory."), 
                    ::error::code::internal::MISC);
                Ok(base.to_path_buf())
            }
        }
    }

    pub fn open(&self) -> UniResult<Box<Read+Seek>> {
        Ok(match *self {
            Input::Stdin => Box::new(stdin()),
            Input::File(ref path) => 
                Box::new(try_!(fs::File::open(path), 
                    path.to_string_lossy().into_owned(), ::error::code::fsio::INPUT))
        })
    }

    pub fn from_args(args: &Args) -> UniResult<Input> {
        if let Some(ref raw_input_path) = args.arg_input {
            let input_path = PathBuf::from(raw_input_path);
            try!(Input::verify_input_path(&input_path));
            Ok(Input::File(input_path))
        }
        else {
            Ok(Input::Stdin)
        }
    }

    fn verify_input_path(file_path: &Path) -> UniResult<()> {
        if ! try_!(fs::metadata(file_path),file_path.to_string_lossy().into_owned(), 
            ::error::code::fsio::INPUT).is_file() {
            return Err(error::usage(format!("Input path must be file: {}", 
                file_path.display())).with_minor(error::code::usage::INPUT_NOT_A_FILE));
        }
        Ok(())
    }
}

pub enum Output {
    /// Destination file, Temporary file, create backup
    InPlace(PathBuf, PathBuf, bool),
    /// Destination file
    OtherFile(PathBuf),
    /// Stdout
    Stdout
}

impl Output {
    fn open_path(path: &PathBuf) -> UniResult<Box<Write>> {
        Ok(Box::new(try_!(fs::File::create(path), 
                    path.to_string_lossy().into_owned(), ::error::code::fsio::OUTPUT)))
    }

    pub fn open(&self) -> UniResult<Box<Write>> {
        match *self {
            Output::InPlace(_,ref tmp_path, _) => Output::open_path(tmp_path),
            Output::OtherFile(ref path) => Output::open_path(path),
            Output::Stdout => Ok(Box::new(stdout()))
        }
    }

    /// Closes stream and performs cleanup work. Expects to be returned the stream that was 
    /// opened before.
    pub fn close(&self, mut file: Box<Write>) -> UniResult<()> {
        // Close the stream before we perform cleanup operations
        try!(file.flush());
        ::std::mem::drop(file);

        match *self {
            Output::Stdout | Output::OtherFile(_) => (),
            Output::InPlace(ref dest_path, ref tmp_path, backup) => 
                try!(Output::close_in_place(dest_path, tmp_path, backup))
        }

        Ok(())
    }

    fn close_in_place(dest_path: &PathBuf, tmp_path: &PathBuf, backup: bool) -> UniResult<()> {
        if backup {
            let mut backup_path = dest_path.clone();
            let mut file_name : ::std::ffi::OsString = try_!(backup_path.file_name()
                .ok_or("Destination path does not have file name component."), 
                ::error::code::internal::MISC).to_os_string();
            file_name.push(".bak");
            backup_path.set_file_name(file_name);
            info!("Backup path: {}", backup_path.display());

            // Perform backup via an atomic replacement operation. 
            // Existing file silently overwritten
            debug!("Creating backup of {} as {} (overwriting any existing files)", 
                dest_path.display(), backup_path.display());
            try_!(::atomicwrites::replace_atomic(dest_path, &backup_path), 
                dest_path.to_string_lossy().into_owned(), 
                ::error::code::fsio::OUTPUT_BACKUP);
        }
        else {
            debug!("No backup for in-place update of {}", dest_path.display());
        }

        debug!("Moving temp output file into place.");
        from_result_!(::atomicwrites::replace_atomic(tmp_path, dest_path), 
            dest_path.to_string_lossy().into_owned(), 
            ::error::code::fsio::OUTPUT)
    }

    fn check_output_path(raw_path: &str, args: &Args) -> UniResult<Output> {
        let some_path = PathBuf::from(raw_path);

        // Check if type of destination and whether it is valid
        let (dir_path, opt_file_path) = match fs::metadata(&some_path) {
            Ok(some_stat) => {
                if some_stat.is_dir() {
                    (some_path, None)      
                } else if some_stat.is_file() {
                    (some_path.parent().expect("File path should have parent dir.").to_path_buf(), 
                        Some(some_path))    
                } else {
                    return Err(from_!(
                        format!("Output path is neither a file nor a directory: {}", raw_path), 
                        error::code::internal::MISC));
                }
            },
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    // Didn't find the file. That is perfectly ok if the destination file doesn't
                    // exist yet. If so, make sure that the directory exists instead.
                    let dir_path = try_!(some_path.parent().ok_or(e), 
                        some_path.to_string_lossy().into_owned(), ::error::code::fsio::OUTPUT)
                        .to_path_buf();
                    let dir_stat = try_!(fs::metadata(&dir_path), 
                        some_path.to_string_lossy().into_owned(), ::error::code::fsio::OUTPUT);
                    if dir_stat.is_dir() {
                        (dir_path.to_path_buf(), Some(some_path))
                    } else {
                        return Err(error::usage(format!("Illegal output path: {}", raw_path))
                            .with_minor(error::code::internal::MISC));
                    }
                } else {
                    return Err(from_!(e, some_path.to_string_lossy().into_owned(), 
                        ::error::code::fsio::OUTPUT));
                }
            }
        };

        // If only a directory is given, derive the file name from the input, if possible
        let file_path = if let Some(f) = opt_file_path { 
            f
        } else  {
            let opt_derived_file_path = args.arg_input.as_ref().and_then(|raw_input| 
                PathBuf::from(raw_input).file_name().map(|file_name| 
                    dir_path.with_file_name(file_name)));

            let r = opt_derived_file_path.ok_or_else(|| error::usage(
                "Input file name needs to be known when no output file name is given.".to_string())
                .with_minor(error::code::usage::MISSING_OUTPUT_FILE_NAME));
            try!(r)
        };

        Ok(Output::OtherFile(file_path))
    }

    pub fn from_args(args: &Args) -> UniResult<Output> {
        if args.flag_stdout {
            Ok(Output::Stdout)
        } else if let Some(ref raw_path) = args.arg_output {
            Output::check_output_path(raw_path, args)
        } else if let Some(ref raw_input_path) = args.arg_input {
            let file_path : PathBuf = PathBuf::from(raw_input_path);
            try!(Input::verify_input_path(&file_path));
            let mut tmp_name = ::std::ffi::OsString::from(".~");
            {
                let file_name = file_path.file_name()
                    .expect("Input file path should have file name.");
                tmp_name.push(file_name);
            }
            tmp_name.push(".tmp");
            let tmp_path = file_path.with_file_name(tmp_name);
            Ok(Output::InPlace(file_path, tmp_path, !args.flag_no_backup))
        } else {
            Err(error::usage(
                "Input file needs to be specified at the very least (for an in-place conversion)."
                .to_owned())
                .with_minor(error::code::usage::MISSING_OUTPUT))
        }
    }
}

pub struct Configuration {
    input: Input,
    output: Output,
    patterns: HashMap<String, String>,
    #[allow(dead_code)]
    raw_args: Args,
    #[allow(dead_code)]
    raw_config: Yaml
} 

impl Configuration {
    fn open_config_file(input: &Input, args: &Args) -> UniResult<(File, PathBuf)> {
        let mut dir_path : PathBuf = try!(input.directory());
        let config_file_name = ::std::ffi::OsString::from(&args.flag_config_name);
        loop {
            let mut config_file_candidate = dir_path.clone();
            config_file_candidate.push(&config_file_name);
            match fs::File::open(&config_file_candidate) {
                Ok(f) => {
                    info!("Found configuration file {:?} as {}", config_file_name, 
                        config_file_candidate.display());
                    return Ok((f, config_file_candidate));
                },
                Err(e)  => {
                    if e.kind() == io::ErrorKind::NotFound {
                        debug!("Configuration file {:?} not found at {}", config_file_name, 
                            config_file_candidate.display());
                        // continue search
                    } else {
                        return Err(from_!(e, config_file_candidate.to_string_lossy().to_string(), 
                            error::code::fsio::CONFIG));    
                    }
                }                
            }

            // Try parent directory. Yes we need the temporary variable, otherwise the Rust 
            // compiler cannot prove that dir_path can be safely overwritten.
            let old_dir_path = dir_path;
            if let Some(parent_path) = old_dir_path.parent() {
                dir_path = parent_path.to_path_buf();
            }
            else {
                return Err(error::usage(format!(
                        "No configuration file {} found searching from {} upwards.", 
                        config_file_name.to_string_lossy(), 
                        input.directory().unwrap_or_else(|_| 
                            PathBuf::from("unknown-file")).display()))
                    .with_minor(error::code::usage::NO_CONFIG_FILE));
            }
        }
    }

    fn read_config_file(config_file_fd: &mut File, config_file_path: &Path) -> UniResult<Yaml> {
        // Need to read the entire YAML file into memeory because the char-streaming-ability of 
        // the std::io::Reader is not stable yet.

        let mut raw_config_text = String::new();
        try_!(config_file_fd.read_to_string(&mut raw_config_text), 
            config_file_path.to_string_lossy().to_string(), error::code::fsio::CONFIG);
        
        let mut docs = try_!(::yaml::YamlLoader::load_from_str(&raw_config_text),
            config_file_path.to_string_lossy().to_string());

        if docs.len() == 0 {
            return Err(error::usage(format!("Expected at least one document in config file {}",
                config_file_path.display())).with_minor(error::code::usage::INVALID_CONFIG_FILE));
        }

        Ok(docs.swap_remove(0))
    }

    fn parse_pattern_entry(raw_key: &Yaml, raw_value: &Yaml, config_file_path: &Path) 
            -> UniResult<(String,String)> {
         let key = match *raw_key {
            Yaml::String(ref key) => key.to_string(),
            ref other => { 
                return Err(error::usage(format!(concat!("Error in configuration file {} ",
                    "Expected string key, instead got: {:?}"), config_file_path.display(), other))
                    .with_minor(error::code::usage::INVALID_CONFIG_FILE));
            }
        };

        let value = match *raw_value {
            Yaml::String(ref value) => value.to_string(),
            ref other => {
                return Err(error::usage(format!(concat!("Error in configuration file {} ",
                    "Expected value of key {} to be a string. Instead got: {:?}"),
                    config_file_path.display(), key, other))
                .with_minor(error::code::usage::INVALID_CONFIG_FILE));
            }
        };

        Ok((key, value))
    }

    fn parse_config(raw_config: &Yaml, config_file_path: &Path, 
            patterns: &mut HashMap<String, String>) -> UniResult<()> {
        let pattern_key = Yaml::String("patterns".to_string());
        if let Yaml::Hash(ref top_level) = *raw_config {
            if let Yaml::Hash(ref raw_pats) = top_level[&pattern_key] {
                for (k,v) in raw_pats {
                    let (key,value) = try!(
                        Configuration::parse_pattern_entry(k, v, config_file_path));
                    debug!("Adding mapping {} -> {}", key, value);
                    patterns.insert(key,value);
                }
                Ok(())
            } else {
                Err(error::usage(format!(concat!(
                    "Expected top-level dictionary of config file {} to contain a dictionary ",
                    "called 'patterns'."), 
                    config_file_path.display()))
                .with_minor(error::code::usage::INVALID_CONFIG_FILE))
            }
        } else {
            Err(
                error::usage(format!("Expected top-level of config file {} to be a dictionary.", 
                    config_file_path.display()))
                .with_minor(error::code::usage::INVALID_CONFIG_FILE))
        }
    }

    /// Creates a Configuration from command line arguments. 
    /// This function accesses the file system in order to validate options and to 
    /// load configuration files.
    /// The arguments are preserved as part of the Configuration data structure.
    pub fn from_args(args: Args) -> UniResult<Configuration> {
        let input = try!(Input::from_args(&args));
        let output = try!(Output::from_args(&args));
        let (mut config_file_fd, config_file_path) = 
            try!(Configuration::open_config_file(&input, &args));
        let raw_config = try!(
            Configuration::read_config_file(&mut config_file_fd, &config_file_path));
        let mut patterns = HashMap::new();
        try!(Configuration::parse_config(&raw_config, &config_file_path, &mut patterns));

        Ok(Configuration {
            input: input, output: output, raw_config: raw_config, patterns: patterns, 
            raw_args: args
        })
    }
}
