use clap::{App, Arg};
use evtx::EvtxParser;
use simple_logger::SimpleLogger;
use std::path::PathBuf;

use log;
use std::io::Write;

mod bodyfilevisitor;
use bodyfilevisitor::BodyfileVisitor;

fn main() {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Warn
    };
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Trace)
        .init();

    SimpleLogger::new()
        .with_level(log_level)
        .with_colors(true)
        .init()
        .unwrap();
        
    let app = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("EVTXFILES")
                .help("names of the evtx files")
                .required(true)
                .multiple(true)
                .min_values(1)
                .takes_value(true),
        );

    let matches = app.get_matches();
    let files: Vec<_> = matches.values_of("EVTXFILES").unwrap().collect();

    for file in files {
        let fp = PathBuf::from(file);
        match EvtxParser::from_path(fp) {
            Ok(mut parser) => {
                for record in parser.records_to_visitor(|| BodyfileVisitor::new()) {
                    match record {
                        Ok(r) => println!("{}", r),
                        Err(e) => {
                            log::error!("{}", e)
                        },
                    }
                }
            }
            Err(error) => {
                log::error!("Error while parsing {}: {}", file, error);
            }
        }
    }
}
