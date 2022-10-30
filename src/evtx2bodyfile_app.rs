use std::path::PathBuf;

use crate::bf_data::*;
use anyhow::Result;
use clap::Parser;
use evtx::{EvtxParser, SerializedEvtxRecord};
use getset::Getters;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use serde_json::Value;

#[derive(Parser, Clone, Getters)]
#[clap(author, version, about, long_about = None)]
pub(crate) struct Evtx2BodyfileApp {
    /// names of the evtx files
    evtx_files: Vec<String>,

    /// output json for elasticsearch instead of bodyfile
    #[clap(short('J'), long("json"))]
    json_output: bool,

    #[clap(flatten)]
    #[getset(get = "pub (crate)")]
    verbose: clap_verbosity_flag::Verbosity,
}

impl Evtx2BodyfileApp {
    pub(crate) fn handle_evtx_files(&self) -> Result<()> {
        for file in self.evtx_files.iter() {
            self.handle_evtx_file(file)?;
        }
        Ok(())
    }

    fn handle_evtx_file(&self, file: &str) -> Result<()> {
        let fp = PathBuf::from(file);
        let bar = Self::create_progress_bar(file)?;
        let mut parser = EvtxParser::from_path(&fp)?;

        for record_r in parser.records_json_value() {
            match record_r {
                Err(why) => log::warn!("{}", why),
                Ok(value) => self.print_record(&value)?
            }
            bar.inc(1);
        }
        bar.finish_and_clear();
        Ok(())
    }

    fn count_records(file: &str) -> Result<usize> {
        let fp = PathBuf::from(file);
        let mut parser = EvtxParser::from_path(&fp)?;
        Ok(parser.serialized_records(|r| r.and(Ok(()))).count())
    }

    fn create_progress_bar(file: &str) -> Result<ProgressBar> {
        let count = Self::count_records(file)?;

        let fp = PathBuf::from(file);
        let filename = fp.file_name().unwrap().to_str().unwrap().to_owned();

        let bar = ProgressBar::new(count as u64);
        let target = ProgressDrawTarget::stderr_with_hz(10);
        bar.set_draw_target(target);
        bar.set_message(filename);

        let progress_style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>9}/{len:9}({percent}%) {msg}")?
            .progress_chars("##-");
        bar.set_style(progress_style);

        Ok(bar)
    }

    fn print_record(&self, record: &SerializedEvtxRecord<Value>) -> Result<()> {
        let mut bf_data: BfData = record.try_into()?;
        bf_data.set_enable_json_output(self.json_output);

        match TryInto::<String>::try_into(bf_data) {
            Err(why) => log::warn!("{}", why),
            Ok(line) => println!("{}", line),
        }
        Ok(())
    }
}
