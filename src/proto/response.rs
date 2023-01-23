use std::{io, str, time::Duration};

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
    BacklightTimeout(Duration),
    DevicePowerOff(Duration),
    Operator(String),
    Company(String),
    Site(String),
    Contact(String),
    Clock(u64),
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
        let value = str::from_utf8(value.as_ref())
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
