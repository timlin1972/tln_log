use std::fmt;

use anstream::println;
use chrono::{DateTime, Local};
use owo_colors::colors::xterm::Gray;
use owo_colors::OwoColorize as _;
use serde::{Deserialize, Serialize};

use common::{cfg, plugin, utils};

const MODULE: &str = "logs";
const MAX_LOGS: usize = 500;

#[derive(Debug)]
struct Log {
    ts: u64,
    log: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Report {
    topic: String,
    payload: String,
}

impl fmt::Display for Log {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let datetime_local: DateTime<Local> = DateTime::from_timestamp(self.ts as i64, 0)
            .unwrap()
            .with_timezone(&Local);

        writeln!(
            f,
            "{}{} {}",
            datetime_local.format("%Y-%m-%d %H:%M:%S %:z").fg::<Gray>(),
            ":".fg::<Gray>(),
            self.log
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Plugin {
    tx: crossbeam_channel::Sender<String>,
    logs: Vec<Log>,
}

impl fmt::Display for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (idx, log) in self.logs.iter().enumerate() {
            write!(f, "\t{}: {}", idx + 1, log)?;
        }
        Ok(())
    }
}

impl Plugin {
    pub fn new(tx: &crossbeam_channel::Sender<String>) -> Plugin {
        println!("[{}] Loading...", MODULE.blue());

        Plugin {
            logs: vec![],
            tx: tx.clone(),
        }
    }
}

impl plugin::Plugin for Plugin {
    fn name(&self) -> &str {
        MODULE
    }

    fn status(&mut self) -> String {
        println!("[{}]", MODULE.blue());

        let mut status = String::new();

        status += &format!("{}", self);

        println!("{status}");

        status
    }

    fn send(&mut self, action: &str, data: &str) -> String {
        match action {
            "report" => {
                if data == "myself" {
                    let report = Report {
                        topic: format!("tln/{}/logs", cfg::get_name()),
                        payload: self.status(),
                    };
                    let json_string = serde_json::to_string(&report).unwrap();

                    self.tx
                        .send(format!("send plugin mqtt report '{json_string}'"))
                        .unwrap();
                }
            }
            "add" => {
                let log = utils::decrypt(data);
                self.logs.push(Log {
                    ts: utils::get_ts(),
                    log,
                });

                if self.logs.len() > MAX_LOGS {
                    self.logs.drain(0..self.logs.len() - MAX_LOGS);
                }
            }
            "clear" => {
                self.logs.clear();
            }
            _ => (),
        }

        "send".to_owned()
    }

    fn unload(&mut self) -> String {
        println!("[{}] Unload", MODULE.blue());

        "unload".to_owned()
    }
}

#[no_mangle]
pub extern "C" fn create_plugin(
    tx: &crossbeam_channel::Sender<String>,
) -> *mut plugin::PluginWrapper {
    let plugin = Box::new(Plugin::new(tx));
    Box::into_raw(Box::new(plugin::PluginWrapper::new(plugin)))
}

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn unload_plugin(wrapper: *mut plugin::PluginWrapper) {
    if !wrapper.is_null() {
        unsafe {
            let _ = Box::from_raw(wrapper);
        }
    }
}
