//! Turns a Source File into DynoWatt's own types. The only module that knows
//! about `fitparser` (ADR 0003): swapping decoder touches nothing else.

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::fmt;

use chrono::{DateTime, Utc};
use fitparser::de::{DecodeOption, from_bytes_with_options};
use fitparser::profile::MesgNum;
use fitparser::{FitDataRecord, Value};
use sha2::{Digest, Sha256};

use crate::db::FitFileId;

/// What a Source File holds, in DynoWatt's terms.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedActivity {
    /// First key of Duplicate detection.
    pub fit_file_id: FitFileId,
    /// SHA-256 of the whole file, lowercase hex: the second key of Duplicate
    /// detection.
    pub content_hash: String,
}

/// Why a Source File cannot become an Activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Not a readable FIT file: failed CRC, truncated or garbled data.
    Unreadable(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Unreadable(reason) => write!(f, "unreadable FIT file: {reason}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decodes the bytes of a Source File.
pub fn decode(bytes: &[u8]) -> Result<DecodedActivity, DecodeError> {
    // Enums come back as their FIT numbers, not as names that may change
    // between profile versions.
    let options = HashSet::from([DecodeOption::ReturnNumericEnumValues]);
    let messages = from_bytes_with_options(bytes, &options)
        .map_err(|e| DecodeError::Unreadable(e.to_string()))?;
    Ok(DecodedActivity {
        fit_file_id: file_id(&messages),
        content_hash: content_hash(bytes),
    })
}

fn content_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn file_id(messages: &[FitDataRecord]) -> FitFileId {
    messages
        .iter()
        .find(|m| m.kind() == MesgNum::FileId)
        .map(|m| FitFileId {
            manufacturer: int(m, "manufacturer").and_then(|v| v.try_into().ok()),
            serial_number: int(m, "serial_number").and_then(|v| v.try_into().ok()),
            time_created: instant(m, "time_created"),
        })
        .unwrap_or_default()
}

fn value<'a>(message: &'a FitDataRecord, name: &str) -> Option<&'a Value> {
    message
        .fields()
        .iter()
        .find(|f| f.name() == name)
        .map(|f| f.value())
}

fn int(message: &FitDataRecord, name: &str) -> Option<i64> {
    value(message, name).and_then(|v| v.try_into().ok())
}

/// A FIT `date_time` field, which counts UTC seconds.
fn instant(message: &FitDataRecord, name: &str) -> Option<DateTime<Utc>> {
    match value(message, name) {
        Some(Value::Timestamp(t)) => Some(t.to_utc()),
        _ => None,
    }
}
