use evtx::err::{SerializationResult, SerializationError};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::HashMap;
use serde_json::json;
use evtx::EvtxStructureVisitor;
use bodyfile::Bodyfile3Line;

pub struct BodyfileVisitor {
    stack: Vec<String>,
    event_id: String,
    provider_name: String,
	channel_name: String,
    timestamp: i64,
    event_data: HashMap<String, String>,
    event_data_name: Option<String>,
    activity_id: Option<String>
}

impl BodyfileVisitor {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            event_id: "".to_owned(),
            provider_name: "".to_owned(),
			channel_name: "".to_owned(),
            timestamp: 0,
            event_data: HashMap::new(),
            event_data_name: None,
            activity_id: None,
        }
    }
}
impl EvtxStructureVisitor for BodyfileVisitor {
    type VisitorResult = Bodyfile3Line;

    fn get_result(
        &self,
        _event_record_id: u64,
        _timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self::VisitorResult {
        let name = 
        match self.activity_id {
            Some(ref activity_id) => format!("Channel={}, Provider={}(EventID={}): Data={} ActivityId={}",
									self.channel_name,
                                    self.provider_name,
                                    self.event_id,
                                    json!(self.event_data),
                                    activity_id),
            None                  => format!("Channel={}, Provider={}(EventID={}): Data={} ActivityId=None",
									self.channel_name,
                                    self.provider_name,
                                    self.event_id,
                                    json!(self.event_data)),
        };

        Bodyfile3Line::new().with_owned_name(name).with_crtime(self.timestamp)
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
            if ! value.is_empty() {
                self.event_data.insert(name.to_owned(), str::replace(value, "|", "§"));
            }
            self.event_data_name = None;
        } else
        if let Some(current_tag) = self.stack.last() {
            if current_tag == "EventID" {
                self.event_id = value.to_owned();
            } else
			if current_tag == "Channel" {
                self.channel_name = value.to_owned();
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
        let date_format = "%Y-%m-%d %H:%M:%S%.f %Z";
        if let Some(parent) = self.stack.last() {

            if parent == "System" {
                if name == "Provider" {
                    self.provider_name = attr_find(attributes, "Name")?.to_owned();
                } else if name == "TimeCreated" {
                    let v = attr_find(attributes, "SystemTime")?;
                    let ndt = match NaiveDateTime::parse_from_str(v, date_format) {
                        Ok(ndt) => ndt,
                        Err(why) => {
                            log::error!("error while parsing '{}': {}", v, why);
                            std::process::exit(-1);
                        }
                    };
                    let dt = DateTime::<Utc>::from_utc(ndt, Utc);
                    self.timestamp = dt.timestamp();

                    assert_eq!(dt.format(date_format).to_string(), v);
                } else if name == "Correlation" {
                    // this attribute might be empty, which would be OK
                    if let Ok(activity_id) = attr_find(attributes, "ActivityID") {
                        self.activity_id = Some(activity_id.to_owned());
                    }
                }

            } else if parent == "EventData" {
                if name == "Data" {
                    self.event_data_name = match attr_find(attributes, "Name") {
                        Ok(n) => Some(n.to_owned()),
                        Err(_) => Some(String::from("binary"))
                    };
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