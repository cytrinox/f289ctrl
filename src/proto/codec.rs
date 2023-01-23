use bytes::BytesMut;
use std::{
    fmt::{self, Write},
    io::{self},
    str,
    time::Duration,
};
use tokio_util::codec::{Decoder, Encoder};

use super::response::{Ident, Response, ResponsePayload};
use crate::proto::command::Command;

#[derive(Default)]
pub struct ProtocolCodec {
    last_cmd: Option<Command>,
}

impl ProtocolCodec {
    pub(crate) fn get_payload(src: &BytesMut) -> Option<Vec<u8>> {
        let offset = src.as_ref().iter().skip(2).position(|b| *b == b'\r');
        if let Some(n) = offset {
            Some(Vec::from(&src[2..n + 2]))
        } else {
            None
        }
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

                        None => panic!("No command called"),
                    }
                }
                '1' => {
                    // Error
                    Ok(Some(Response::SyntaxError))
                }
                '2' => {
                    // Device locked
                    Ok(Some(Response::ExecutionError))
                }
                '5' => {
                    // No data
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
        }
        dst.write_str("\r")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.last_cmd = Some(item);
        Ok(())
    }
}
