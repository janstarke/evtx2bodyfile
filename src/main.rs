use clap::{App, Arg};
use evtx::{EvtxParser, SerializedEvtxRecord};
use serde::Serialize;
use serde_json::{Value, json};
use std::{path::PathBuf, collections::HashMap};
use bodyfile::Bodyfile3Line;
use anyhow::{Result, anyhow};

use log;
use std::io::Write;

fn main() {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Warn
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
        .filter(None, log_level)
        .init();
        
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
                for record_r in parser.records_json_value() {
                    match record_r {
                        Err(why) => log::warn!("{}", why),
                        Ok(value) => match record_to_mactime(value) {
                            Err(why) => log::warn!("{}", why),
                            Ok(line) => println!("{}", line),
                        }
                    }
                }
            }
            Err(error) => {
                log::error!("Error while parsing {}: {}", file, error);
            }
        }
    }
}
#[derive(Serialize)]
struct BfData<'a> {
    event_record_id: u64,
    event_id: &'a Value,
    provider_name: &'a Value,
	channel_name: &'a Value,
    activity_id: Option<&'a Value>,
    custom_data: HashMap<&'a String, &'a Value>
}

macro_rules! from_json {
    ($value: ident, $( $att:expr ),+ ) => {
        {
            let mut value = $value;
            $(
                value = value.get($att).ok_or(anyhow!("missing '{}' key in {}", $att, value))?;
            )+
            value
        }
    };
}

impl<'a> BfData<'a> {
    pub fn from(event_record_id: u64, value: &'a Value) -> Result<Self> {
        let event = from_json!(value, "Event");
        let system = from_json!(event, "System");
        let event_id = {
            let event_id = from_json!(system, "EventID");
            match event_id.get("#text") {
                Some(eid) => eid,
                None => event_id
            }
        };
        

        let provider_name= from_json!(system, "Provider", "#attributes", "Name");
        let channel_name = from_json!(system, "Channel");

        let activity_id = system.get("Correlation")
                                                .and_then(|c|c.get("#attributes"))
                                                .and_then(|c|c.get("ActivityId"));
        
        let mut custom_data = HashMap::new();
        match event {
            Value::Object(contents) => {
                for (key, value) in contents.iter() {
                    if key != "System" && key != "#attributes" {
                        custom_data.insert(key, value);
                    }
                }
            }
            _ => ()
        }
        Ok(Self {
            event_record_id,
            event_id,
            provider_name,
            channel_name,
            activity_id,
            custom_data
        })
    }
}

fn record_to_mactime(record: SerializedEvtxRecord<Value>) -> Result<String> {
    let bf_data = BfData::from(record.event_record_id, &record.data)?;
    let bf_line = Bodyfile3Line::new()
        .with_mtime(record.timestamp.timestamp())
        .with_owned_name(json!(bf_data).to_string());
    Ok(bf_line.to_string())
}
