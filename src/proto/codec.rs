use bytes::BytesMut;
use std::{
    fmt::{self, Write},
    io::{self},
    str,
    time::Duration,
};
use tokio_util::codec::{Decoder, Encoder};

use crate::proto::command::Command;
use crate::{
    device::ValueMap,
    proto::response::{Ident, MemoryStat, Response, ResponsePayload},
    rawmea::{RawMeasurement, RawSavedMeasurement},
    rawmea::{
        RawSavedMinMaxMeasurement, RawSavedPeakMeasurement, RawSavedRecordingSessionInfo,
        RawSessionRecordReadings, BIN_MARKER_LEN, MEA_METADATA_LEN, READING_LEN,
    },
};

use super::command::{
    ClearMemory, DateFormat, DezibelReference, DigitCount, Language, NumericFormat, TimeFormat,
};

const STATUS_LEN: usize = 2;
const EOL_LEN: usize = 1; // one byte for '\r'

#[derive(Default)]
pub struct ProtocolCodec {
    last_cmd: Option<Command>,
}

impl ProtocolCodec {
    pub(crate) fn get_payload(src: &BytesMut) -> Option<Vec<u8>> {
        let offset = src.as_ref().iter().skip(2).position(|b| *b == b'\r');
        offset.map(|n| Vec::from(&src[2..n + 2]))
    }

    fn convert_string(payload: impl AsRef<[u8]>) -> std::io::Result<String> {
        Ok(str::from_utf8(payload.as_ref())
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?
            .to_string())
    }
}

impl Decoder for ProtocolCodec {
    type Item = Response;
    // We use io::Error here instead of our own Error type beacause
    // for the low level protocol, receiving an ExecutionError or the like
    // is totally fine, as the decoding is successful. Deciding if this should
    // be returned as an error is up to a higher level.
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() >= 2 {
            if (src[1] as char) != '\r' {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Device response code expected",
                ));
            }
            match src[0] as char {
                '0' => {
                    // Success

                    match self.last_cmd {
                        Some(Command::SetBacklightTimeout(_))
                        | Some(Command::SetDevicePowerOff(_))
                        | Some(Command::SetOperator(_))
                        | Some(Command::SetCompany(_))
                        | Some(Command::SetSite(_))
                        | Some(Command::SetContact(_))
                        | Some(Command::SetSaveName(_, _))
                        | Some(Command::SetBeeper(_))
                        | Some(Command::SetSmoothing(_))
                        | Some(Command::Clear(_))
                        | Some(Command::ResetDevice)
                        | Some(Command::SetCustomDbm(_))
                        | Some(Command::SetDigitCount(_))
                        | Some(Command::SetAutoHoldEventThreshold(_))
                        | Some(Command::SetRecordingEventThreshold(_))
                        | Some(Command::SetLanguage(_))
                        | Some(Command::SetDateFormat(_))
                        | Some(Command::SetTimeFormat(_))
                        | Some(Command::SetNumFormat(_))
                        | Some(Command::SetDbmRef(_))
                        | Some(Command::SetTempOffset(_))
                        | Some(Command::SetClock(_)) => {
                            let _ = src.split_to(2);
                            Ok(Some(Response::Success(None)))
                        }
                        Some(Command::Id) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ident::try_from(payload.as_slice()).map(|id| {
                                    Some(Response::Success(Some(ResponsePayload::Id(id))))
                                })
                            } else {
                                Ok(None)
                            }
                        }
                        Some(Command::QueryMap(_)) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let _ = src.split_to(2 + payload.len() + 1);

                                let mut value_map = ValueMap::new();

                                let value = str::from_utf8(payload.as_ref())
                                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                                    .to_string();
                                let mut values: Vec<&str> = value.split(',').collect();
                                assert!(!values.is_empty());

                                let c = values[0]
                                    .parse::<usize>()
                                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                                values.drain(0..1);

                                for entry in values.chunks_exact(2) {
                                    let id = entry[0]
                                        .parse::<u16>()
                                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                                    let name = entry[1];
                                    value_map.insert(id, name.to_string());
                                }

                                assert_eq!(c, value_map.len());

                                Ok(Some(Response::Success(Some(ResponsePayload::Map(
                                    value_map,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }
                        Some(Command::GetBacklightTimeout) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let secs = line
                                    .parse::<u64>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::BacklightTimeout(Duration::from_secs(secs)),
                                ))))
                            } else {
                                Ok(None)
                            }
                        }
                        Some(Command::GetDevicePowerOff) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let secs = line
                                    .parse::<u64>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::DevicePowerOff(Duration::from_secs(secs)),
                                ))))
                            } else {
                                Ok(None)
                            }
                        }
                        Some(Command::GetOperator) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::Operator(
                                    strip_string(line),
                                )))))
                            } else {
                                Ok(None)
                            }
                        }
                        Some(Command::GetCompany) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::Company(
                                    strip_string(line),
                                )))))
                            } else {
                                Ok(None)
                            }
                        }
                        Some(Command::GetSite) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::Site(
                                    strip_string(line),
                                )))))
                            } else {
                                Ok(None)
                            }
                        }
                        Some(Command::GetContact) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::Contact(
                                    strip_string(line),
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetClock) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let secs = line
                                    .parse::<u64>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::Clock(secs)))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetBeeper) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let state = line.eq("ON");
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::Beeper(
                                    state,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetSmoothing) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let state = line.eq("ON");
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::Smoothing(
                                    state,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetCustomDbm) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let d_bm = line
                                    .parse::<u16>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::CustomDbm(
                                    d_bm,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetDigitCount) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let digits = line
                                    .parse::<u8>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                let d = match digits {
                                    4 => DigitCount::Digit4,
                                    5 => DigitCount::Digit5,
                                    _ => unimplemented!(),
                                };
                                Ok(Some(Response::Success(Some(ResponsePayload::DigitCount(
                                    d,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetLanguage) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                let lang = match line.as_str() {
                                    "GERMAN" => Language::German,
                                    "ENLISH" => Language::English,
                                    "SPANISH" => Language::Spanish,
                                    "ITALIAN" => Language::Italian,
                                    "FRENCH" => Language::French,
                                    "JAPANESE" => Language::Japanese,
                                    "CHINESE" => Language::Chinese,
                                    _ => unimplemented!(),
                                };
                                Ok(Some(Response::Success(Some(ResponsePayload::Language(
                                    lang,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetDateFormat) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                let fmt = match line.as_str() {
                                    "MM_DD" => DateFormat::MM_DD,
                                    "DD_MM" => DateFormat::DD_MM,
                                    _ => unimplemented!(),
                                };
                                Ok(Some(Response::Success(Some(ResponsePayload::DateFormat(
                                    fmt,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetTimeFormat) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                let v = line
                                    .parse::<u8>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let fmt = match v {
                                    12 => TimeFormat::Time12,
                                    24 => TimeFormat::Time24,
                                    _ => unimplemented!(),
                                };
                                Ok(Some(Response::Success(Some(ResponsePayload::TimeFormat(
                                    fmt,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetNumFormat) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                let fmt = match line.as_str() {
                                    "COMMA" => NumericFormat::Comma,
                                    "POINT" => NumericFormat::Point,
                                    _ => unimplemented!(),
                                };
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::NumericFormat(fmt),
                                ))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetDbmRef) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let d_bm = line
                                    .parse::<u16>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                let x = match d_bm {
                                    0 => DezibelReference::Custom,
                                    _ => unimplemented!(),
                                };
                                Ok(Some(Response::Success(Some(ResponsePayload::DbmRef(x)))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetTempOffset) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let offset = line
                                    .parse::<i16>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::TempOffset(
                                    offset,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetAutoHoldEventThreshold) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let th = line
                                    .parse::<u8>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::AutoHoldEventThreshold(th),
                                ))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetRecordingEventThreshold) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let th = line
                                    .parse::<u8>()
                                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::RecordingEventThreshold(th),
                                ))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetSaveName(_)) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let line = Self::convert_string(&payload)?;
                                let _ = src.split_to(2 + payload.len() + 1);
                                Ok(Some(Response::Success(Some(ResponsePayload::SaveName(
                                    line,
                                )))))
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetMemoryStat) => {
                            if let Some(payload) = Self::get_payload(src) {
                                let _ = src.split_to(2 + payload.len() + 1);
                                MemoryStat::try_from(payload.as_slice()).map(|stat| {
                                    Some(Response::Success(Some(ResponsePayload::MemoryStat(stat))))
                                })
                            } else {
                                Ok(None)
                            }
                        }

                        Some(Command::GetMeasurementBinary) => {
                            if src.len() >= STATUS_LEN + BIN_MARKER_LEN + MEA_METADATA_LEN {
                                let readings: u16 = u16::from_le_bytes([
                                    src[2 + BIN_MARKER_LEN + MEA_METADATA_LEN - 2],
                                    src[2 + BIN_MARKER_LEN + MEA_METADATA_LEN - 1],
                                ]);
                                let total = STATUS_LEN
                                    + BIN_MARKER_LEN
                                    + MEA_METADATA_LEN
                                    + (readings as usize * READING_LEN)
                                    + EOL_LEN;
                                if src.len() >= total {
                                    let m = RawMeasurement::try_from(&src[2..total])?; // Skip STATUS
                                    let _ = src.split_to(total);
                                    return Ok(Some(Response::Success(Some(
                                        ResponsePayload::MeasurementBinary(m),
                                    ))));
                                }
                            }
                            Ok(None) // Not enough bytes yet
                        }

                        Some(Command::QuerySavedMeasurement(_)) => {
                            if let Some(count) = RawSavedMeasurement::can_parse(&src[2..])? {
                                let payload = src.split_to(2 + count);
                                let m = RawSavedMeasurement::try_from(&payload[2..])?;
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::SavedMeasurement(m),
                                ))))
                            } else {
                                Ok(None) // Not enough bytes yet
                            }
                        }

                        Some(Command::QueryMinMaxSessionInfo(_)) => {
                            if let Some(count) = RawSavedMinMaxMeasurement::can_parse(&src[2..])? {
                                let payload = src.split_to(2 + count);
                                let m = RawSavedMinMaxMeasurement::try_from(&payload[2..])?;
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::MinMaxSessionInfo(m),
                                ))))
                            } else {
                                Ok(None) // Not enough bytes yet
                            }

                            /*
                            if src.len() >= STATUS_LEN + BIN_MARKER_LEN + SAVED_MINMAX_METADATA_LEN
                            {
                                let readings: u16 = u16::from_le_bytes([
                                    src[2 + BIN_MARKER_LEN + SAVED_MINMAX_METADATA_LEN - 2],
                                    src[2 + BIN_MARKER_LEN + SAVED_MINMAX_METADATA_LEN - 1],
                                ]);
                                let total = STATUS_LEN
                                    + BIN_MARKER_LEN
                                    + SAVED_MINMAX_METADATA_LEN
                                    + (readings as usize * READING_LEN)
                                    + EOL_LEN;
                                if src.len() >= total {
                                    let m = RawSavedMinMaxMeasurement::try_from(&src[2..total])?; // Skip STATUS
                                    let _ = src.split_to(total); // TODO: test
                                    return Ok(Some(Response::Success(Some(
                                        ResponsePayload::MinMaxSessionInfo(m),
                                    ))));
                                }
                            }
                            Ok(None) // Not enough bytes yet
                             */
                        }

                        Some(Command::QueryPeakSessionInfo(_)) => {
                            if let Some(count) = RawSavedPeakMeasurement::can_parse(&src[2..])? {
                                let payload = src.split_to(2 + count);
                                let m = RawSavedPeakMeasurement::try_from(&payload[2..])?;
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::PeakSessionInfo(m),
                                ))))
                            } else {
                                Ok(None) // Not enough bytes yet
                            }
                            /*
                            if src.len() >= STATUS_LEN + BIN_MARKER_LEN + SAVED_PEAK_METADATA_LEN {
                                let readings: u16 = u16::from_le_bytes([
                                    src[2 + BIN_MARKER_LEN + SAVED_PEAK_METADATA_LEN - 2],
                                    src[2 + BIN_MARKER_LEN + SAVED_PEAK_METADATA_LEN - 1],
                                ]);
                                let total = STATUS_LEN
                                    + BIN_MARKER_LEN
                                    + SAVED_PEAK_METADATA_LEN
                                    + (readings as usize * READING_LEN)
                                    + EOL_LEN;
                                if src.len() >= total {
                                    let m = RawSavedPeakMeasurement::try_from(&src[2..total])?; // Skip STATUS
                                    let _ = src.split_to(total); // TODO: test
                                    return Ok(Some(Response::Success(Some(
                                        ResponsePayload::PeakSessionInfo(m),
                                    ))));
                                }
                            }
                            Ok(None) // Not enough bytes yet
                            */
                        }

                        Some(Command::QueryRecordedSessionInfo(_)) => {
                            if let Some(count) = RawSavedRecordingSessionInfo::can_parse(&src[2..])?
                            {
                                let payload = src.split_to(2 + count);
                                let m = RawSavedRecordingSessionInfo::try_from(&payload[2..])?;
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::RecordedSessionInfo(m),
                                ))))
                            } else {
                                Ok(None) // Not enough bytes yet
                            }
                            /*
                            if src.len()
                                >= STATUS_LEN + BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN
                            {
                                let readings: u16 = u16::from_le_bytes([
                                    src[2 + BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN - 2],
                                    src[2 + BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN - 1],
                                ]);
                                let total = STATUS_LEN
                                    + BIN_MARKER_LEN
                                    + SAVED_RECORDING_METADATA_LEN
                                    + (readings as usize * READING_LEN)
                                    + EOL_LEN;
                                if src.len() >= total {
                                    let m = RawSavedRecordingSessionInfo::try_from(&src[2..total])?; // Skip STATUS
                                    let _ = src.split_to(total); // TODO: test
                                    return Ok(Some(Response::Success(Some(
                                        ResponsePayload::RecordedSessionInfo(m),
                                    ))));
                                }
                            }
                            Ok(None) // Not enough bytes yet
                            */
                        }

                        Some(Command::QuerySessionRecordReadings(_, _)) => {
                            if let Some(count) = RawSessionRecordReadings::can_parse(&src[2..])? {
                                let payload = src.split_to(2 + count);
                                let m = RawSessionRecordReadings::try_from(&payload[2..])?;
                                Ok(Some(Response::Success(Some(
                                    ResponsePayload::SessionRecordReading(m),
                                ))))
                            } else {
                                Ok(None) // Not enough bytes yet
                            }
                        }

                        None => panic!("No command called"),
                    }
                }
                '1' => {
                    // Error
                    let _ = src.split_to(2);
                    Ok(Some(Response::SyntaxError))
                }
                '2' => {
                    // Device locked
                    let _ = src.split_to(2);
                    Ok(Some(Response::ExecutionError))
                }
                '5' => {
                    // No data
                    let _ = src.split_to(2);
                    Ok(Some(Response::NoData))
                }
                code => Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Unknown device response code: {:?}", code),
                )),
            }
        } else {
            Ok(None)
        }
    }
}

fn write_fmt_guarded(dst: &mut BytesMut, args: fmt::Arguments<'_>) -> Result<(), io::Error> {
    dst.write_fmt(args)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

fn strip_string(s: impl AsRef<str>) -> String {
    s.as_ref()
        .chars()
        .skip(1)
        .take(s.as_ref().len() - 2)
        .collect()
}

fn enclose_string(s: impl AsRef<str>) -> String {
    format!("'{}'", s.as_ref()).to_string()
}

impl Encoder<Command> for ProtocolCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Command, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match &item {
            Command::Id => write_fmt_guarded(dst, format_args!("id"))?,
            Command::QueryMap(name) => write_fmt_guarded(dst, format_args!("qemap {}", name))?,
            Command::SetBacklightTimeout(d) => {
                write_fmt_guarded(dst, format_args!("mp ablto,{}", d.as_secs()))?
            }
            Command::GetBacklightTimeout => write_fmt_guarded(dst, format_args!("qmp ablto"))?,
            Command::SetDevicePowerOff(d) => {
                write_fmt_guarded(dst, format_args!("mp apoffto,{}", d.as_secs()))?
            }
            Command::GetDevicePowerOff => write_fmt_guarded(dst, format_args!("qmp apoffto"))?,
            Command::GetOperator => write_fmt_guarded(dst, format_args!("qmpq operator"))?,

            Command::SetOperator(operator) => write_fmt_guarded(
                dst,
                format_args!("mpq operator,{}", enclose_string(operator)),
            )?,
            Command::GetCompany => write_fmt_guarded(dst, format_args!("qmpq company"))?,
            Command::SetCompany(company) => {
                write_fmt_guarded(dst, format_args!("mpq company,{}", enclose_string(company)))?
            }
            Command::GetSite => write_fmt_guarded(dst, format_args!("qmpq site"))?,
            Command::SetSite(site) => {
                write_fmt_guarded(dst, format_args!("mpq site,{}", enclose_string(site)))?
            }
            Command::GetContact => write_fmt_guarded(dst, format_args!("qmpq contact"))?,
            Command::SetContact(contact) => {
                write_fmt_guarded(dst, format_args!("mpq contact,{}", contact))?
            }
            Command::GetClock => write_fmt_guarded(dst, format_args!("qmp clock"))?,
            Command::SetClock(clock) => write_fmt_guarded(dst, format_args!("mp clock,{}", clock))?,
            Command::GetSaveName(slot) => {
                write_fmt_guarded(dst, format_args!("qsavname {}", slot))?
            }
            Command::SetSaveName(slot, name) => write_fmt_guarded(
                dst,
                format_args!("savname {},{}", slot, enclose_string(name)),
            )?,
            Command::GetMemoryStat => write_fmt_guarded(dst, format_args!("qsls"))?,
            Command::GetMeasurementBinary => write_fmt_guarded(dst, format_args!("qddb"))?,
            Command::QuerySavedMeasurement(idx) => {
                write_fmt_guarded(dst, format_args!("qsmr {}", idx))?
            }
            Command::QueryMinMaxSessionInfo(idx) => {
                write_fmt_guarded(dst, format_args!("qmmsi {}", idx))?
            }
            Command::QueryPeakSessionInfo(idx) => {
                write_fmt_guarded(dst, format_args!("qpsi {}", idx))?
            }
            Command::QueryRecordedSessionInfo(idx) => {
                write_fmt_guarded(dst, format_args!("qrsi {}", idx))?
            }
            Command::QuerySessionRecordReadings(reading_idx, sample_idx) => {
                write_fmt_guarded(dst, format_args!("qsrr {},{}", reading_idx, sample_idx))?
            }
            Command::Clear(mem) => {
                let s = match mem {
                    ClearMemory::All => "ALL",
                    ClearMemory::Measurements => "MEASUREMENT",
                    ClearMemory::MinMax => "MIN_MAX",
                    ClearMemory::Peak => "PEAK",
                    ClearMemory::Recordings => "RECORDED",
                };
                write_fmt_guarded(dst, format_args!("csd {}", s))?;
            }
            Command::ResetDevice => write_fmt_guarded(dst, format_args!("rmp"))?,
            Command::GetBeeper => write_fmt_guarded(dst, format_args!("qmp beeper"))?,
            Command::SetBeeper(state) => {
                if *state {
                    write_fmt_guarded(dst, format_args!("mp beeper,ON"))?
                } else {
                    write_fmt_guarded(dst, format_args!("mp beeper,OFF"))?
                }
            }
            Command::GetSmoothing => write_fmt_guarded(dst, format_args!("qmp acsmooth"))?,
            Command::SetSmoothing(state) => {
                if *state {
                    write_fmt_guarded(dst, format_args!("mp acsmooth,ON"))?
                } else {
                    write_fmt_guarded(dst, format_args!("mp acsmooth,OFF"))?
                }
            }
            Command::GetDigitCount => write_fmt_guarded(dst, format_args!("qmp digits"))?,
            Command::SetDigitCount(digits) => {
                let s = match digits {
                    DigitCount::Digit4 => "4",
                    DigitCount::Digit5 => "5",
                };
                write_fmt_guarded(dst, format_args!("mp digits,{}", s))?;
            }
            Command::GetLanguage => write_fmt_guarded(dst, format_args!("qmp lang"))?,
            Command::SetLanguage(lang) => {
                let s = match lang {
                    Language::German => "GERMAN",
                    Language::English => "ENGLISH",
                    Language::French => "FRENCH",
                    Language::Italian => "ITALIAN",
                    Language::Spanish => "SPANISH",
                    Language::Japanese => "JAPANESE",
                    Language::Chinese => "CHINESE",
                };
                write_fmt_guarded(dst, format_args!("mp lang,{}", s))?;
            }
            Command::GetDateFormat => write_fmt_guarded(dst, format_args!("qmp dateFmt"))?,
            Command::SetDateFormat(fmt) => {
                let s = match fmt {
                    DateFormat::DD_MM => "DD_MM",
                    DateFormat::MM_DD => "MM_DD",
                };
                write_fmt_guarded(dst, format_args!("mp dateFmt,{}", s))?;
            }
            Command::GetTimeFormat => write_fmt_guarded(dst, format_args!("qmp timeFmt"))?,
            Command::SetTimeFormat(fmt) => {
                let s = match fmt {
                    TimeFormat::Time12 => "12",
                    TimeFormat::Time24 => "24",
                };
                write_fmt_guarded(dst, format_args!("mp timeFmt,{}", s))?;
            }
            Command::GetNumFormat => write_fmt_guarded(dst, format_args!("qmp numFmt"))?,
            Command::SetNumFormat(fmt) => {
                let s = match fmt {
                    NumericFormat::Point => "POINT",
                    NumericFormat::Comma => "COMMA",
                };
                write_fmt_guarded(dst, format_args!("mp numFmt,{}", s))?;
            }
            Command::GetAutoHoldEventThreshold => {
                write_fmt_guarded(dst, format_args!("qmp ahEventTh"))?
            }
            Command::SetAutoHoldEventThreshold(thd) => {
                write_fmt_guarded(dst, format_args!("mp ahEventTh,{}", thd))?;
            }
            Command::GetRecordingEventThreshold => {
                write_fmt_guarded(dst, format_args!("qmp recEventTh"))?
            }
            Command::SetRecordingEventThreshold(thd) => {
                write_fmt_guarded(dst, format_args!("mp recEventTh,{}", thd))?;
            }
            Command::GetCustomDbm => write_fmt_guarded(dst, format_args!("qmp cusDBm"))?,
            Command::SetCustomDbm(d_bm) => {
                write_fmt_guarded(dst, format_args!("mp cusDBm,{}", d_bm))?;
            }
            Command::GetDbmRef => write_fmt_guarded(dst, format_args!("qmp dBmRef"))?,
            Command::SetDbmRef(d_bm) => {
                let param = match d_bm {
                    super::command::DezibelReference::Ref4 => "4",
                    super::command::DezibelReference::Ref8 => "8",
                    super::command::DezibelReference::Ref16 => "16",
                    super::command::DezibelReference::Ref25 => "25",
                    super::command::DezibelReference::Ref32 => "32",
                    super::command::DezibelReference::Ref50 => "50",
                    super::command::DezibelReference::Ref75 => "75",
                    super::command::DezibelReference::Ref600 => "600",
                    super::command::DezibelReference::Ref1000 => "1000",
                    super::command::DezibelReference::Custom => "0",
                };
                write_fmt_guarded(dst, format_args!("mp dBmRef,{}", param))?;
            }
            Command::GetTempOffset => write_fmt_guarded(dst, format_args!("qmp tempOs"))?,
            Command::SetTempOffset(offset) => {
                write_fmt_guarded(dst, format_args!("mp tempOs,{}", offset))?;
            }
        }
        dst.write_str("\r")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.last_cmd = Some(item);
        Ok(())
    }
}
