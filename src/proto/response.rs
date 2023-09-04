use std::{io, str, time::Duration};

use crate::{
    device::ValueMap,
    rawmea::{
        RawMeasurement, RawSavedMeasurement, RawSavedMinMaxMeasurement, RawSavedPeakMeasurement,
        RawSavedRecordingSessionInfo, RawSessionRecordReadings,
    },
};

use super::command::{
    DateFormat, DezibelReference, DigitCount, Language, NumericFormat, TimeFormat,
};

/// Device response is build by an ASCII status code
/// and a CARRIGDE RETURN (0x13).
/// For commands returning a data line, the line
/// follows the status response line.
#[derive(Debug, Clone)]
pub enum Response {
    Success(Option<ResponsePayload>), // 0
    SyntaxError,                      // 1
    ExecutionError,                   // 2
    NoData,                           // 5
}

#[derive(Debug, Clone)]
pub enum ResponsePayload {
    Id(Ident),
    Map(ValueMap),
    BacklightTimeout(Duration),
    DevicePowerOff(Duration),
    Operator(String),
    Company(String),
    Site(String),
    Contact(String),
    Clock(u64),
    Beeper(bool),
    Smoothing(bool),
    SaveName(String),
    MemoryStat(MemoryStat),
    MeasurementBinary(RawMeasurement),
    SavedMeasurement(RawSavedMeasurement),

    MinMaxSessionInfo(RawSavedMinMaxMeasurement),

    PeakSessionInfo(RawSavedPeakMeasurement),
    RecordedSessionInfo(RawSavedRecordingSessionInfo),
    SessionRecordReading(RawSessionRecordReadings),

    CustomDbm(u16),
    DigitCount(DigitCount),
    AutoHoldEventThreshold(u8),
    RecordingEventThreshold(u8),
    Language(Language),
    DateFormat(DateFormat),
    TimeFormat(TimeFormat),
    NumericFormat(NumericFormat),
    DbmRef(DezibelReference),
    TempOffset(i16),
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub model: String,
    pub firmware: String,
    pub serial: String,
}

impl TryFrom<&[u8]> for Ident {
    type Error = io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let value = str::from_utf8(value)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .to_string();
        let values: Vec<&str> = value.split(',').collect();
        if values.len() == 3 {
            Ok(Self {
                model: String::from(values[0]),
                firmware: String::from(values[1]),
                serial: String::from(values[2]),
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Invalid data for ID response: {}", value),
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStat {
    pub recordings: usize,
    pub min_max: usize,
    pub peak: usize,
    pub measurement: usize,
}

impl TryFrom<&[u8]> for MemoryStat {
    type Error = io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let value = str::from_utf8(value)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .to_string();
        let values: Vec<&str> = value.split(',').collect();
        if values.len() == 4 {
            Ok(Self {
                recordings: values[0]
                    .parse::<usize>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
                min_max: values[1]
                    .parse::<usize>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
                peak: values[2]
                    .parse::<usize>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
                measurement: values[3]
                    .parse::<usize>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Invalid data for qsls response: {}", value),
            ))
        }
    }
}
