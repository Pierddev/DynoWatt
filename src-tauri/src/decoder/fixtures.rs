//! Builds synthetic `.FIT` files for the decoder tests with `rustyfit`'s
//! encoder (a dev-dependency only), so no personal ride ever lands in the
//! repo: real rides reveal where the Athlete lives.

use chrono::{DateTime, Utc};
use embedded_io_adapters::std::FromStd;
use rustyfit::Encoder;
use rustyfit::profile::mesgdef;
use rustyfit::profile::typedef::{File, FitBaseType, MesgNum};
use rustyfit::proto::{FIT, Field, Message, Value};
use std::io::Cursor;

pub use rustyfit::profile::typedef::{Event, EventType, Sport, SubSport};

/// FIT timestamps count seconds from 1989-12-31T00:00:00Z.
const FIT_EPOCH_UNIX: i64 = 631_065_600;

fn fit_time(t: DateTime<Utc>) -> u32 {
    u32::try_from(t.timestamp() - FIT_EPOCH_UNIX).expect("a time after the FIT epoch")
}

/// The values of one `record` message; `None` leaves the field out.
#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub power_w: Option<u16>,
    pub heart_rate_bpm: Option<u8>,
    pub cadence_rpm: Option<u8>,
    pub altitude_m: Option<f64>,
    pub speed_m_s: Option<f64>,
    pub distance_m: Option<f64>,
}

/// The `file_id` triple a device writes.
#[derive(Debug, Clone, Copy)]
pub struct FileId {
    pub manufacturer: u16,
    pub serial_number: u32,
    pub time_created: DateTime<Utc>,
}

/// A FIT activity file under construction. Messages are written in the order
/// they are added, after a leading `file_id`.
#[derive(Default)]
pub struct FitBuilder {
    file_id: Option<FileId>,
    messages: Vec<Message>,
}

fn field(num: u8, base_type: FitBaseType, value: Value) -> Field {
    Field {
        num,
        base_type,
        value,
        is_expanded: false,
    }
}

fn message(num: MesgNum, fields: Vec<Field>) -> Message {
    Message {
        num,
        fields,
        ..Default::default()
    }
}

/// Raw value of a scaled FIT field: `(value + offset) * scale`.
fn scaled(value: f64, scale: f64, offset: f64) -> Value {
    Value::Uint32(((value + offset) * scale).round() as u32)
}

impl FitBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn file_id(mut self, file_id: FileId) -> Self {
        self.file_id = Some(file_id);
        self
    }

    /// A `session` message, where devices record the sport.
    pub fn session(mut self, sport: Sport, sub_sport: SubSport) -> Self {
        self.messages.push(message(
            MesgNum::SESSION,
            vec![
                field(
                    mesgdef::Session::SPORT,
                    FitBaseType::ENUM,
                    Value::Uint8(sport.0),
                ),
                field(
                    mesgdef::Session::SUB_SPORT,
                    FitBaseType::ENUM,
                    Value::Uint8(sub_sport.0),
                ),
            ],
        ));
        self
    }

    /// A `sport` message, the other place a sport can be recorded.
    pub fn sport(mut self, sport: Sport, sub_sport: SubSport) -> Self {
        self.messages.push(message(
            MesgNum::SPORT,
            vec![
                field(
                    mesgdef::Sport::SPORT,
                    FitBaseType::ENUM,
                    Value::Uint8(sport.0),
                ),
                field(
                    mesgdef::Sport::SUB_SPORT,
                    FitBaseType::ENUM,
                    Value::Uint8(sub_sport.0),
                ),
            ],
        ));
        self
    }

    /// A `record` message at `at`.
    pub fn record(mut self, at: DateTime<Utc>, point: Point) -> Self {
        let mut fields = vec![field(
            mesgdef::Record::TIMESTAMP,
            FitBaseType::UINT32,
            Value::Uint32(fit_time(at)),
        )];
        if let Some(v) = point.power_w {
            fields.push(field(
                mesgdef::Record::POWER,
                FitBaseType::UINT16,
                Value::Uint16(v),
            ));
        }
        if let Some(v) = point.heart_rate_bpm {
            fields.push(field(
                mesgdef::Record::HEART_RATE,
                FitBaseType::UINT8,
                Value::Uint8(v),
            ));
        }
        if let Some(v) = point.cadence_rpm {
            fields.push(field(
                mesgdef::Record::CADENCE,
                FitBaseType::UINT8,
                Value::Uint8(v),
            ));
        }
        if let Some(v) = point.altitude_m {
            fields.push(field(
                mesgdef::Record::ENHANCED_ALTITUDE,
                FitBaseType::UINT32,
                scaled(v, 5.0, 500.0),
            ));
        }
        if let Some(v) = point.speed_m_s {
            fields.push(field(
                mesgdef::Record::ENHANCED_SPEED,
                FitBaseType::UINT32,
                scaled(v, 1000.0, 0.0),
            ));
        }
        if let Some(v) = point.distance_m {
            fields.push(field(
                mesgdef::Record::DISTANCE,
                FitBaseType::UINT32,
                scaled(v, 100.0, 0.0),
            ));
        }
        self.messages.push(message(MesgNum::RECORD, fields));
        self
    }

    /// A timer `event` message: start, stop, stop_all…
    pub fn timer(self, at: DateTime<Utc>, event_type: EventType) -> Self {
        self.event(at, Event::TIMER, event_type)
    }

    /// An `event` message of any kind: timer, lap…
    pub fn event(mut self, at: DateTime<Utc>, event: Event, event_type: EventType) -> Self {
        self.messages.push(message(
            MesgNum::EVENT,
            vec![
                field(
                    mesgdef::Event::TIMESTAMP,
                    FitBaseType::UINT32,
                    Value::Uint32(fit_time(at)),
                ),
                field(
                    mesgdef::Event::EVENT,
                    FitBaseType::ENUM,
                    Value::Uint8(event.0),
                ),
                field(
                    mesgdef::Event::EVENT_TYPE,
                    FitBaseType::ENUM,
                    Value::Uint8(event_type.0),
                ),
            ],
        ));
        self
    }

    /// A `record` message with the 16-bit `altitude` and `speed` fields older
    /// devices write instead of the `enhanced_` ones.
    pub fn legacy_record(mut self, at: DateTime<Utc>, altitude_m: f64, speed_m_s: f64) -> Self {
        let raw =
            |v: f64, scale: f64, offset: f64| Value::Uint16(((v + offset) * scale).round() as u16);
        self.messages.push(message(
            MesgNum::RECORD,
            vec![
                field(
                    mesgdef::Record::TIMESTAMP,
                    FitBaseType::UINT32,
                    Value::Uint32(fit_time(at)),
                ),
                field(
                    mesgdef::Record::ALTITUDE,
                    FitBaseType::UINT16,
                    raw(altitude_m, 5.0, 500.0),
                ),
                field(
                    mesgdef::Record::SPEED,
                    FitBaseType::UINT16,
                    raw(speed_m_s, 1000.0, 0.0),
                ),
            ],
        ));
        self
    }

    /// Encodes the file, header and CRCs included.
    pub fn build(self) -> Vec<u8> {
        let mut file_id = vec![field(
            mesgdef::FileId::TYPE,
            FitBaseType::ENUM,
            Value::Uint8(File::ACTIVITY.0),
        )];
        if let Some(id) = self.file_id {
            file_id.extend([
                field(
                    mesgdef::FileId::MANUFACTURER,
                    FitBaseType::UINT16,
                    Value::Uint16(id.manufacturer),
                ),
                field(
                    mesgdef::FileId::SERIAL_NUMBER,
                    FitBaseType::UINT32Z,
                    Value::Uint32(id.serial_number),
                ),
                field(
                    mesgdef::FileId::TIME_CREATED,
                    FitBaseType::UINT32,
                    Value::Uint32(fit_time(id.time_created)),
                ),
            ]);
        }
        let mut fit = FIT {
            messages: std::iter::once(message(MesgNum::FILE_ID, file_id))
                .chain(self.messages)
                .collect(),
            ..Default::default()
        };
        let mut out = FromStd::new(Cursor::new(Vec::new()));
        Encoder::new()
            .encode(&mut out, &mut fit)
            .expect("a valid synthetic FIT file");
        out.into_inner().into_inner()
    }
}
