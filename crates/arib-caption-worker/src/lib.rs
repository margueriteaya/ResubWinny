use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

mod archive;
#[path = "../../../shared/arib_symbols.rs"]
mod arib_symbols;
mod arib_text;
mod caption;
#[path = "../../../shared/caption_features.rs"]
mod caption_features;
mod cli;
mod config;
mod drcs;
mod export_assessment;
mod exporters;
#[cfg(feature = "fuzzing")]
pub mod fuzzing;
mod input;
mod inspection;
mod jobs;
mod models;
mod native_b24;
#[cfg(feature = "libaribtlv")]
#[allow(dead_code)] // PoC boundary; production TLV routing remains evidence-first.
mod native_tlv;
mod preview;
mod protocol;
mod resource;
pub mod synthetic;
mod time;
mod timeline;
mod transport;

pub(crate) use archive::*;
pub(crate) use caption::*;
pub(crate) use config::*;
pub(crate) use drcs::*;
pub(crate) use export_assessment::*;
pub(crate) use exporters::*;
pub(crate) use inspection::*;
pub(crate) use jobs::*;
pub(crate) use models::*;
pub(crate) use preview::*;
pub(crate) use protocol::*;
pub(crate) use resource::*;
pub(crate) use time::*;
pub(crate) use timeline::*;
pub(crate) use transport::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    match cli::run() {
        Ok(()) => Ok(()),
        Err(error) => {
            let conflict = error
                .downcast_ref::<io::Error>()
                .and_then(|error| error.get_ref())
                .and_then(|error| error.downcast_ref::<ExportConflict>());
            if let Some(conflict) = conflict {
                protocol::emit_export_conflict(conflict);
            } else {
                protocol::emit_failed("worker.operation_failed", &error.to_string());
            }
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests;
