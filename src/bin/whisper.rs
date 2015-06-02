extern crate graphite;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate time;

extern crate regex;

use docopt::Docopt;
use graphite::whisper;
use graphite::whisper::schema::RetentionPolicy;
use std::process::exit;

static USAGE: &'static str = "
Usage:
    whisper info <file>
    whisper update <file> <timestamp> <value>
    whisper mark <file> <value>
    whisper thrash <file> <value> <times>
    whisper create <file> <timespec>...

Options:
    --xff <x_files_factor>
    --aggregation_method <method>
";

#[derive(RustcDecodable, Debug)]
struct Args {
    cmd_info: bool,
    cmd_update: bool,
    cmd_mark: bool,
    cmd_thrash: bool,
    cmd_create: bool,

    arg_file: String,
    arg_timestamp: String,
    arg_value: String,
    arg_times: String,

    arg_timespec: Vec<String>
}


pub fn main(){
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let arg_file = args.arg_file.clone();
    let path = unsafe {
        arg_file.slice_unchecked(0, args.arg_file.len())
    };

    let current_time = time::get_time().sec as u64;

    if args.cmd_info {
        let file = whisper::file::open(path).unwrap();
        println!("{:?}", file);
    } else if args.cmd_update {
        cmd_update(args, path, current_time);
    } else if args.cmd_mark {
        cmd_mark(args, path, current_time);
    } else if args.cmd_thrash {
        cmd_thrash(args, path, current_time);
    } else if args.cmd_create {
        cmd_create(args, path);
    } else {
        panic!("must specify command");
    }
}

fn cmd_update(args: Args, path: &str, current_time: u64) {
    let mut file = whisper::file::open(path).unwrap();
    let point = whisper::point::Point{
        timestamp: args.arg_timestamp.parse::<u64>().unwrap(),
        value: args.arg_value.parse::<f64>().unwrap()
    };
    debug!("Updating TS: {} with value: {}", point.timestamp, point.value);

    file.write(current_time, point);
}

fn cmd_mark(args: Args, path: &str, current_time: u64) {
    let mut file = whisper::file::open(path).unwrap();
    let point = whisper::point::Point{
        timestamp: current_time,
        value: args.arg_value.parse::<f64>().unwrap()
    };

    file.write(current_time, point);
}

fn cmd_thrash(args: Args, path: &str, current_time: u64) {
    let times = args.arg_times.parse::<u64>().unwrap();
    let mut file = whisper::file::open(path).unwrap();
    for index in 1..times {
        let point = whisper::point::Point{
            timestamp: current_time+index,
            value: args.arg_value.parse::<f64>().unwrap()
        };

        file.write(current_time+index, point);
    }
}

fn mult_str_to_num(mult_str: &str) -> u64 {
    match mult_str {
        "s" => 1,
        "m" => 60,
        "h" => 60*60,
        "d" => 60*60*24,
        "w" => 60*60*24*7,
        "y" => 60*60*24*365,
        _   => {
            // should never pass regex
            println!("All retention policies must be valid. Exiting.");
            exit(1);
        }
    }
}

fn retention_capture_to_pair(regex_match: regex::Captures) -> Option<whisper::schema::RetentionPolicy> {
    let precision_opt = regex_match.at(1);
    let precision_mult = regex_match.at(2).unwrap_or("s");
    let points_opt = regex_match.at(3);
    let points_mult = regex_match.at(4).unwrap_or("s");

    if precision_opt.is_some() && points_opt.is_some() {
        let precision = precision_opt.unwrap().parse::<u64>().unwrap();
        let points = points_opt.unwrap().parse::<u64>().unwrap();

        let retention_spec = whisper::schema::RetentionPolicy {
            precision: precision * mult_str_to_num(precision_mult),
            points: points * mult_str_to_num(points_mult)
        };

        Some(retention_spec)
    } else {
        None
    }
}

fn parse_spec_to_retention_policy(spec: &str) -> Option<RetentionPolicy> {
    // TODO: regex should be built as const using macro regex!
    // but that's only available in nightlies.
    let retention_matcher = regex::Regex::new({r"^(\d+)([smhdwy])?:(\d+)([smhdwy])?$"}).unwrap();
    match retention_matcher.captures(spec) {
        Some(regex_match) => {
            retention_capture_to_pair(regex_match)
        },
        None => None
    }
}

fn validate_retention_policies(expanded_pairs: &Vec<(&String, &Option<RetentionPolicy>)> ) {
        let _ : Vec<()> = expanded_pairs.iter().map(|pair: &(&String, &Option<RetentionPolicy>)| {
            let (ref string, ref opt) = *pair;
            if opt.is_none() {
                println!("error: {} is not a valid retention policy", string);
                exit(1);
            }
        }).collect();
}

fn cmd_create(args: Args, _: &str) {
    
    let retention_policies : Vec<RetentionPolicy> = {
        let specs : Vec<String> = args.arg_timespec;

        let expanded_pairs : Vec<Option<RetentionPolicy>> = specs.iter().map(|ts| {
            parse_spec_to_retention_policy(ts)
        }).collect();

        if expanded_pairs.iter().any(|x| x.is_none()) {
            let specs_iter = specs.iter();
            let pairs_iter = expanded_pairs.iter();
            let error_pairs : Vec<(&String, &Option<RetentionPolicy>)> = specs_iter.zip(pairs_iter).collect();
            validate_retention_policies(&error_pairs);
        }

        expanded_pairs.iter().filter(|x| x.is_some()).map(|x| x.unwrap()).collect()
    };

    println!("retention_policies: {:?}", retention_policies);
}