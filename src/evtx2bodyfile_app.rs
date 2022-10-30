
use crate::{bf_data::*, evtx_file::EvtxFile};
use anyhow::Result;
use clap::Parser;
use evtx::SerializedEvtxRecord;
use getset::Getters;
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
            self.handle_evtx_file((&file[..]).try_into()?)?;
        }
        Ok(())
    }

    fn handle_evtx_file(&self, evtx_file: EvtxFile) -> Result<()> {       
        let bar = evtx_file.create_progress_bar()?;
        for value in evtx_file.into_iter() {
            self.print_record(&value)?;
            bar.inc(1);
        }
        bar.finish_and_clear();
        Ok(())
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
