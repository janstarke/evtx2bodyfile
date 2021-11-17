use clap::{App, Arg};
use evtx::EvtxParser;
use simple_logger::SimpleLogger;
use std::path::PathBuf;
use log;

mod bodyfilevisitor;
use bodyfilevisitor::BodyfileVisitor;

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
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
                        Err(e) => log::error!("{}", e),
                    }
                }
            }
            Err(error) => {
                log::error!("Error while parsing {}: {}", file, error);
            }
        }
    }
}
