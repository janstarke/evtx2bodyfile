use evtx::err::{SerializationResult, SerializationError};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::HashMap;
use serde_json::json;
use evtx::EvtxStructureVisitor;
use crate::BodyfileLine;

pub struct BodyfileVisitor {
    stack: Vec<String>,
    event_id: String,
    provider_name: String,
    timestamp: i64,
    event_data: HashMap<String, String>,
    event_data_name: Option<String>
}

impl BodyfileVisitor {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            event_id: "".to_owned(),
            provider_name: "".to_owned(),
            timestamp: 0,
            event_data: HashMap::new(),
            event_data_name: None,
        }
    }
}
impl EvtxStructureVisitor for BodyfileVisitor {
    type VisitorResult = BodyfileLine;

    fn get_result(
        &self,
        _event_record_id: u64,
        _timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self::VisitorResult {
        let name = format!("{}({}): {}",
            self.provider_name,
            self.event_id,
            json!(self.event_data)
        );

        BodyfileLine::new(name, self.timestamp)
    }

    /// called when a new record starts
    fn start_record(&mut self) -> SerializationResult<()> {
        Ok(())
    }

    /// called when the current records is finished
    fn finalize_record(&mut self) -> SerializationResult<()> {
        Ok(())
    }

    // called upon element content
    fn visit_characters(&mut self, value: &str) -> SerializationResult<()> {
        if let Some(ref name) = self.event_data_name {
            self.event_data.insert(name.to_owned(), str::replace(value, "|", "§"));
            self.event_data_name = None;
        } else
        if let Some(current_tag) = self.stack.last() {
            if current_tag == "EventID" {
                self.event_id = value.to_owned();
            }
        }
        Ok(())
    }

    /// called when a complex element (i.e. an element with child elements) starts
    fn visit_start_element<'a, 'b, I>(
        &'a mut self,
        name: &'b str,
        attributes: I,
    ) -> SerializationResult<()>
    where
        'a: 'b,
        I: Iterator<Item = (&'b str, &'b str)> + 'b,
    {
        if let Some(parent) = self.stack.last() {

            if parent == "System" {
                if name == "Provider" {
                    self.provider_name = attr_find(attributes, "Name")?.to_owned();
                } else if name == "TimeCreated" {
                    let v = attr_find(attributes, "SystemTime")?;
                    let ndt = NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S%.f %Z").unwrap();
                    let dt = DateTime::<Utc>::from_utc(ndt, Utc);
                    self.timestamp = dt.timestamp();
                }

            } else if parent == "EventData" {
                if name == "Data" {
                    self.event_data_name = Some(attr_find(attributes, "Name")?.to_owned());
                }
            }
        }

        self.stack.push(name.to_owned());
        Ok(())
    }

    /// called when a complex element (i.e. an element with child elements) ends
    fn visit_end_element(&mut self, _name: &str) -> SerializationResult<()> {
        self.stack.pop();
        self.event_data_name = None;
        Ok(())
    }
}

fn attr_find<'a, 'b, I>(attributes: I, key: &str) -> SerializationResult<&'b str> where I:Iterator<Item = (&'b str, &'b str)> + 'b {
    for (k, v) in attributes {
        if k == key {
            return Ok(v);
        }
    }
    Err(SerializationError::ExternalError {
        cause: format!("no value found for key '{}'", key)
    })
}