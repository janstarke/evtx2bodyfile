use anyhow::{anyhow, Result, bail};
use bodyfile::Bodyfile3Line;
use chrono::{DateTime, Utc};
use clap::Parser;
use es4forensics::objects::WindowsEvent;
use evtx::{EvtxParser, SerializedEvtxRecord};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use serde::Serialize;
use serde_json::{json, Value};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::{collections::HashMap, path::PathBuf};

#[derive(Parser, Clone)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// names of the evtx files
    evtx_files: Vec<String>,

    /// output json for elasticsearch instead of bodyfile
    #[clap(short('J'), long("json"))]
    json_output: bool,

    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    TermLogger::init(
        cli.verbose.log_level_filter(),
        Config::default(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )?;

    for file in cli.evtx_files.iter() {
        let fp = PathBuf::from(file);
        let count = match EvtxParser::from_path(&fp) {
            Err(why) => {
                log::error!("Error while parsing {}: {}", file, why);
                continue;
            }
            Ok(mut parser) => parser.serialized_records(|r| r.and(Ok(()))).count(),
        };
        let filename = fp.file_name().unwrap().to_str().unwrap().to_owned();
        match EvtxParser::from_path(&fp) {
            Ok(mut parser) => {
                let bar = ProgressBar::new(count as u64);
                let target = ProgressDrawTarget::stderr_with_hz(10);
                bar.set_draw_target(target);
                bar.set_message(filename);

                let progress_style = ProgressStyle::default_bar()
                    .template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>9}/{len:9}({percent}%) {msg}",
                    )?
                    .progress_chars("##-");
                bar.set_style(progress_style);

                for record_r in parser.records_json_value() {
                    match record_r {
                        Err(why) => log::warn!("{}", why),

                        Ok(value) => {
                            let res = if cli.json_output {
                                record_to_json(value)
                            } else {
                                record_to_mactime(value)
                            };
                            match res {
                                Err(why) => log::warn!("{}", why),
                                Ok(line) => println!("{}", line),
                            }
                        }
                    }
                    bar.inc(1);
                }
                bar.finish_and_clear();
            }
            Err(error) => {
                log::error!("Error while parsing {}: {}", file, error);
            }
        }
    }

    Ok(())
}
#[derive(Serialize)]
struct BfData<'a> {
    event_record_id: u64,
    timestamp: DateTime<Utc>,
    event_id: &'a Value,
    provider_name: &'a Value,
    channel_name: &'a Value,
    activity_id: Option<&'a Value>,
    custom_data: HashMap<&'a String, &'a Value>,
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

impl<'a> TryFrom<&'a SerializedEvtxRecord<Value>> for BfData<'a> {
    type Error = anyhow::Error;

    fn try_from(record: &'a SerializedEvtxRecord<Value>) -> Result<Self, Self::Error> {
        let value = &record.data;
        let event = from_json!(value, "Event");
        let system = from_json!(event, "System");
        let event_id = {
            let event_id = from_json!(system, "EventID");
            match event_id.get("#text") {
                Some(eid) => eid,
                None => event_id,
            }
        };

        let provider_name = from_json!(system, "Provider", "#attributes", "Name");
        let channel_name = from_json!(system, "Channel");

        let activity_id = system
            .get("Correlation")
            .and_then(|c| c.get("#attributes"))
            .and_then(|c| c.get("ActivityId"));

        let mut custom_data = HashMap::new();
        if let Value::Object(contents) = event {
            for (key, value) in contents.iter() {
                if key != "System" && key != "#attributes" {
                    custom_data.insert(key, value);
                }
            }
        }

        Ok(Self {
            event_record_id: record.event_record_id,
            timestamp: record.timestamp,
            event_id,
            provider_name,
            channel_name,
            activity_id,
            custom_data,
        })
    }
}

impl<'a> From<BfData<'a>> for WindowsEvent<'a> {
    fn from(bfdata: BfData<'a>) -> Self {
        WindowsEvent::new(
            bfdata.event_record_id,
            bfdata.timestamp,
            bfdata.event_id.as_u64().unwrap(),
            bfdata.provider_name,
            bfdata.channel_name,
            bfdata.activity_id,
            bfdata.custom_data.clone()
        )
    }
}

fn record_to_mactime(record: SerializedEvtxRecord<Value>) -> Result<String> {
    let bf_data: BfData = (&record).try_into()?;
    let bf_line = Bodyfile3Line::new()
        .with_mtime(record.timestamp.timestamp())
        .with_owned_name(json!(bf_data).to_string());
    Ok(bf_line.to_string())
}

fn record_to_json(record: SerializedEvtxRecord<Value>) -> Result<String> {
    let bf_data: BfData = (&record).try_into()?;
    
    let event_id = match bf_data.event_id.as_u64() {
        Some(id) => id,
        None => return Err(anyhow!("event_id has no valid value")),
    };

    let event: WindowsEvent = bf_data.into();
    match event.documents().next() {
        None => bail!("missing value for this record"),
        Some((_ts, value)) => {
            match serde_json::to_string(&value) {
                Ok(s) => Ok(s),
                Err(why) => Err(anyhow!(why))
            }
        }
    }
}
