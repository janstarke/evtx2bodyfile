use clap::Parser;
use evtx::{EvtxParser, SerializedEvtxRecord};
use serde::Serialize;
use serde_json::{Value, json};
use simplelog::{TermLogger, Config, TerminalMode, ColorChoice};
use std::{path::PathBuf, collections::HashMap};
use bodyfile::Bodyfile3Line;
use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser, Clone)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// names of the evtx files
    evtx_files: Vec<String>,

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
            Ok(mut parser) => {
                parser.serialized_records(|r| r.and(Ok(()))).count()
            }
        };
        let filename = fp.file_name().unwrap().to_str().unwrap().to_owned();
        match EvtxParser::from_path(&fp) {
            Ok(mut parser) => {
                let bar = ProgressBar::new(count as u64);
                bar.set_draw_delta(100);
                bar.set_message(filename);

                let progress_style = ProgressStyle::default_bar()
                        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>9}/{len:9}({percent}%) {msg}")
                        .progress_chars("##-");
                bar.set_style(progress_style);

                for record_r in parser.records_json_value() {
                    match record_r {
                        Err(why) => log::warn!("{}", why),
                        Ok(value) => match record_to_mactime(value) {
                            Err(why) => log::warn!("{}", why),
                            Ok(line) => println!("{}", line),
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
        if let Value::Object(contents) = event {
            for (key, value) in contents.iter() {
                if key != "System" && key != "#attributes" {
                    custom_data.insert(key, value);
                }
            }
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
