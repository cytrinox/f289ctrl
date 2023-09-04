use byteorder::{LittleEndian, ReadBytesExt};
use bytes::Buf;
use std::io::BufRead;
use std::io::Cursor;
use std::io::Read;

pub(crate) const BIN_MARKER_LEN: usize = 2;

pub(crate) const MEA_METADATA_LEN: usize = 34;
pub(crate) const SAVED_MEA_METADATA_LEN: usize = 38;
pub(crate) const SAVED_MINMAX_METADATA_LEN: usize = 54;
//pub(crate) const SAVED_PEAK_METADATA_LEN: usize = SAVED_MINMAX_METADATA_LEN;
pub(crate) const SAVED_RECORDING_METADATA_LEN: usize = 78;
pub(crate) const SAVED_RECORD_READINGS_LEN: usize = 26 + READING_LEN + (3 * READING_LEN); // = 146

pub(crate) const READING_LEN: usize = 30;

pub(crate) const EOL_LEN: usize = 1;

#[derive(Debug, Clone)]
pub struct RawMeasurement {
    pub pri_function: u16,
    pub sec_function: u16,
    pub auto_range: u16,
    pub unit: u16,
    pub range_max: f64,
    pub unit_multiplier: i16,
    pub bolt: u16,
    pub ts: f64,
    pub modes: u16,
    pub un1: u16,
    pub readings: Vec<RawReading>,
}

impl TryFrom<&[u8]> for RawMeasurement {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        assert!(value.len() >= BIN_MARKER_LEN + MEA_METADATA_LEN);

        if value[0..2] == [b'#', b'0'] {
            let mut cur = Cursor::new(&value[2..]);

            let pri_function = cur.read_u16::<LittleEndian>()?;
            let sec_function = cur.read_u16::<LittleEndian>()?;
            let auto_range = cur.read_u16::<LittleEndian>()?;
            let unit = cur.read_u16::<LittleEndian>()?;
            let range_max = read_double(&mut cur)?;
            let unit_multiplier = cur.read_i16::<LittleEndian>()?;
            let bolt = cur.read_u16::<LittleEndian>()?;
            let ts = cur.read_f64::<LittleEndian>()?;
            let mode = cur.read_u16::<LittleEndian>()?;
            let un1 = cur.read_u16::<LittleEndian>()?;
            let readings_cnt = cur.read_u16::<LittleEndian>()?;

            let mut readings = Vec::with_capacity(readings_cnt as usize);

            assert_eq!(cur.remaining(), readings_cnt as usize * READING_LEN + 1);

            for _ in 0..readings_cnt {
                let mut buf = [0; READING_LEN];
                cur.read_exact(&mut buf)?;
                let reading = RawReading::try_from(&buf[..])?;
                readings.push(reading);
            }

            Ok(RawMeasurement {
                pri_function,
                sec_function,
                auto_range,
                unit,
                range_max,
                unit_multiplier,
                bolt,
                ts,
                modes: mode,
                un1,
                readings,
            })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Binary data expected but not #0 marker found",
            ))
        }
    }
}

fn read_double(buf: &mut Cursor<&[u8]>) -> std::result::Result<f64, std::io::Error> {
    let mut data = [0_u8; 8];
    buf.read_exact(&mut data)?;
    data.swap(0, 3);
    data.swap(1, 2);
    data.swap(4, 7);
    data.swap(5, 6);
    Ok(f64::from_be_bytes(data))
}

#[derive(Debug, Clone)]
pub struct RawReading {
    pub reading_id: u16,
    pub value: f64,
    pub unit: u16,
    pub unit_multiplier: i16,
    pub decimals: i16,
    pub display_digits: i16,
    pub state: u16,
    pub attribute: u16,
    pub ts: f64,
}

impl TryFrom<&[u8]> for RawReading {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        let mut cur = Cursor::new(value);

        let reading_id = cur.read_u16::<LittleEndian>()?;
        let value = read_double(&mut cur)?;
        let unit = cur.read_u16::<LittleEndian>()?;
        let unit_multiplier = cur.read_i16::<LittleEndian>()?;
        let decimals = cur.read_i16::<LittleEndian>()?;
        let display_digits = cur.read_i16::<LittleEndian>()?;
        let state = cur.read_u16::<LittleEndian>()?;
        let attribute = cur.read_u16::<LittleEndian>()?;
        let ts = read_double(&mut cur)?;

        Ok(RawReading {
            reading_id,
            value,
            unit,
            unit_multiplier,
            decimals,
            display_digits,
            state,
            attribute,
            ts,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RawSavedMeasurement {
    pub seq_no: u16,
    pub un1: u16,
    pub pri_function: u16,
    pub sec_function: u16,
    pub auto_range: u16,
    pub unit: u16,
    pub range_max: f64,
    pub unit_multiplier: i16,
    pub bolt: u16,
    pub un2: u16,
    pub un3: u16,
    pub un4: u16,
    pub un5: u16,
    pub modes: u16,
    pub un6: u16,

    pub readings: Vec<RawReading>,

    pub name: String,
}

impl RawSavedMeasurement {
    pub fn can_parse(buf: &[u8]) -> std::io::Result<Option<usize>> {
        if buf.len() >= BIN_MARKER_LEN + SAVED_MEA_METADATA_LEN {
            // readings count is on last two bytes
            let readings: u16 = u16::from_le_bytes([
                buf[BIN_MARKER_LEN + SAVED_MEA_METADATA_LEN - 2],
                buf[BIN_MARKER_LEN + SAVED_MEA_METADATA_LEN - 1],
            ]);
            // how many bytes total before ASCII data
            let total = BIN_MARKER_LEN + SAVED_MEA_METADATA_LEN + (readings as usize * READING_LEN);

            if buf.len() > total {
                if let Some(idx) = buf[total..].iter().position(|b| *b == b'\r') {
                    return Ok(Some(total + idx + EOL_LEN));
                }
            }
        }
        Ok(None) // Not enough data yet
    }
}

fn read_saved_name(cur: &mut Cursor<&[u8]>) -> std::io::Result<String> {
    assert!(cur.has_remaining(), "Need more bytes for name");
    let mut name_buf = Vec::with_capacity(30);
    cur.read_until(b'\r', &mut name_buf)?;
    assert_eq!(name_buf.last(), Some(&b'\r'));
    name_buf.pop(); // remove delimiter
    Ok(String::from_utf8_lossy(name_buf.as_ref()).to_string())
}

impl TryFrom<&[u8]> for RawSavedMeasurement {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        assert!(value.len() >= BIN_MARKER_LEN + SAVED_MEA_METADATA_LEN);

        if value[0..2] == [b'#', b'0'] {
            let mut cur = Cursor::new(&value[2..]);

            let seq_no = cur.read_u16::<LittleEndian>()?;
            let un1 = cur.read_u16::<LittleEndian>()?;
            let pri_function = cur.read_u16::<LittleEndian>()?;
            let sec_function = cur.read_u16::<LittleEndian>()?;
            let auto_range = cur.read_u16::<LittleEndian>()?;
            let unit = cur.read_u16::<LittleEndian>()?;
            let range_max = read_double(&mut cur)?;
            let unit_multiplier = cur.read_i16::<LittleEndian>()?;
            let bolt = cur.read_u16::<LittleEndian>()?;

            let un2 = cur.read_u16::<LittleEndian>()?;
            let un3 = cur.read_u16::<LittleEndian>()?;
            let un4 = cur.read_u16::<LittleEndian>()?;
            let un5 = cur.read_u16::<LittleEndian>()?;

            let mode = cur.read_u16::<LittleEndian>()?;

            let un6 = cur.read_u16::<LittleEndian>()?;

            let readings_cnt = cur.read_u16::<LittleEndian>()?;

            let mut readings = Vec::with_capacity(readings_cnt as usize);

            //assert_eq!(cur.remaining(), readings_cnt as usize * READING_LEN + 1);

            for _ in 0..readings_cnt {
                let mut buf = [0; READING_LEN];
                cur.read_exact(&mut buf)?;
                let reading = RawReading::try_from(&buf[..])?;
                readings.push(reading);
            }

            let name = read_saved_name(&mut cur)?;

            Ok(RawSavedMeasurement {
                seq_no,
                un1,
                pri_function,
                sec_function,
                auto_range,
                unit,
                range_max,
                unit_multiplier,
                bolt,
                un2,
                un3,
                un4,
                un5,
                modes: mode,
                un6,
                readings,
                name,
            })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Binary data expected but not #0 marker found",
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawSavedMinMaxMeasurement {
    pub seq_no: u16,
    pub un1: u16,
    pub ts1: f64,
    pub ts2: f64,
    pub pri_function: u16,
    pub sec_function: u16,
    pub auto_range: u16,
    pub unit: u16,
    pub range_max: f64,
    pub unit_multiplier: i16,
    pub bolt: u16,
    pub ts3: f64,
    pub modes: u16,
    pub un2: u16,

    pub readings: Vec<RawReading>,

    pub name: String,
}

impl RawSavedMinMaxMeasurement {
    pub fn can_parse(buf: &[u8]) -> std::io::Result<Option<usize>> {
        if buf.len() >= BIN_MARKER_LEN + SAVED_MINMAX_METADATA_LEN {
            // readings count is on last two bytes
            let readings: u16 = u16::from_le_bytes([
                buf[BIN_MARKER_LEN + SAVED_MINMAX_METADATA_LEN - 2],
                buf[BIN_MARKER_LEN + SAVED_MINMAX_METADATA_LEN - 1],
            ]);
            // how many bytes total before ASCII data
            let total =
                BIN_MARKER_LEN + SAVED_MINMAX_METADATA_LEN + (readings as usize * READING_LEN);

            if buf.len() > total {
                if let Some(idx) = buf[total..].iter().position(|b| *b == b'\r') {
                    return Ok(Some(total + idx + EOL_LEN));
                }
            }
        }
        Ok(None) // Not enough data yet
    }
}

impl TryFrom<&[u8]> for RawSavedMinMaxMeasurement {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        assert!(value.len() >= BIN_MARKER_LEN + SAVED_MINMAX_METADATA_LEN);

        if value[0..2] == [b'#', b'0'] {
            let mut cur = Cursor::new(&value[2..]);

            let seq_no = cur.read_u16::<LittleEndian>()?;
            let un1 = cur.read_u16::<LittleEndian>()?;
            let ts1 = read_double(&mut cur)?;
            let ts2 = read_double(&mut cur)?;
            let pri_function = cur.read_u16::<LittleEndian>()?;
            let sec_function = cur.read_u16::<LittleEndian>()?;
            let auto_range = cur.read_u16::<LittleEndian>()?;
            let unit = cur.read_u16::<LittleEndian>()?;
            let range_max = read_double(&mut cur)?;
            let unit_multiplier = cur.read_i16::<LittleEndian>()?;
            let bolt = cur.read_u16::<LittleEndian>()?;
            let ts3 = read_double(&mut cur)?;
            let mode = cur.read_u16::<LittleEndian>()?;
            let un2 = cur.read_u16::<LittleEndian>()?;

            let readings_cnt = cur.read_u16::<LittleEndian>()?;

            let mut readings = Vec::with_capacity(readings_cnt as usize);

            //assert_eq!(cur.remaining(), readings_cnt as usize * READING_LEN + 1);

            for _ in 0..readings_cnt {
                let mut buf = [0; READING_LEN];
                cur.read_exact(&mut buf)?;
                let reading = RawReading::try_from(&buf[..])?;
                readings.push(reading);
            }

            let name = read_saved_name(&mut cur)?;

            Ok(RawSavedMinMaxMeasurement {
                seq_no,
                un1,
                ts1,
                ts2,
                pri_function,
                sec_function,
                auto_range,
                unit,
                range_max,
                unit_multiplier,
                bolt,
                ts3,
                modes: mode,
                un2,
                readings,
                name,
            })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Binary data expected but not #0 marker found",
            ))
        }
    }
}

// Same structure
pub type RawSavedPeakMeasurement = RawSavedMinMaxMeasurement;

#[derive(Debug, Clone)]
pub struct RawSavedRecordingSessionInfo {
    pub seq_no: u16,
    pub un1: u16,
    pub start_ts: f64,
    pub end_ts: f64,
    pub sample_interval: f64,
    pub event_threshold: f64,
    pub reading_index: u16,
    pub un2: u16,
    pub num_samples: u16,
    pub un3: u16,
    pub pri_function: u16,
    pub sec_function: u16,
    pub auto_range: u16,
    pub unit: u16,
    pub range_max: f64,
    pub unit_multiplier: i16,
    pub bolt: u16,
    pub un4: u16,
    pub un5: u16,
    pub un6: u16,
    pub un7: u16,
    pub modes: u16,
    pub un8: u16,

    pub readings: Vec<RawReading>,

    pub name: String,
}

impl TryFrom<&[u8]> for RawSavedRecordingSessionInfo {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        assert!(value.len() >= BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN);

        if value[0..2] == [b'#', b'0'] {
            let mut cur = Cursor::new(&value[2..]);

            let seq_no = cur.read_u16::<LittleEndian>()?;
            let un1 = cur.read_u16::<LittleEndian>()?;
            let start_ts = read_double(&mut cur)?;
            let end_ts = read_double(&mut cur)?;
            let sample_interval = read_double(&mut cur)?;
            let event_threshold = read_double(&mut cur)?;
            let reading_index = cur.read_u16::<LittleEndian>()?;
            let un2 = cur.read_u16::<LittleEndian>()?;
            let num_samples = cur.read_u16::<LittleEndian>()?;
            let un3 = cur.read_u16::<LittleEndian>()?;
            let pri_function = cur.read_u16::<LittleEndian>()?;
            let sec_function = cur.read_u16::<LittleEndian>()?;
            let auto_range = cur.read_u16::<LittleEndian>()?;
            let unit = cur.read_u16::<LittleEndian>()?;
            let range_max = read_double(&mut cur)?;
            let unit_multiplier = cur.read_i16::<LittleEndian>()?;
            let bolt = cur.read_u16::<LittleEndian>()?;
            let un4 = cur.read_u16::<LittleEndian>()?;
            let un5 = cur.read_u16::<LittleEndian>()?;
            let un6 = cur.read_u16::<LittleEndian>()?;
            let un7 = cur.read_u16::<LittleEndian>()?;
            let mode = cur.read_u16::<LittleEndian>()?;
            let un8 = cur.read_u16::<LittleEndian>()?;

            let readings_cnt = cur.read_u16::<LittleEndian>()?;

            let mut readings = Vec::with_capacity(readings_cnt as usize);

            //assert_eq!(cur.remaining(), readings_cnt as usize * READING_LEN + 1);

            for _ in 0..readings_cnt {
                let mut buf = [0; READING_LEN];
                cur.read_exact(&mut buf)?;
                let reading = RawReading::try_from(&buf[..])?;
                readings.push(reading);
            }

            let name = read_saved_name(&mut cur)?;

            Ok(RawSavedRecordingSessionInfo {
                seq_no,
                un1,
                start_ts,
                end_ts,
                sample_interval,
                event_threshold,
                reading_index,
                un2,
                num_samples,
                un3,
                pri_function,
                sec_function,
                auto_range,
                unit,
                range_max,
                unit_multiplier,
                bolt,
                un4,
                un5,
                un6,
                un7,
                modes: mode,
                un8,
                readings,
                name,
            })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Binary data expected but not #0 marker found",
            ))
        }
    }
}

impl RawSavedRecordingSessionInfo {
    pub fn can_parse(buf: &[u8]) -> std::io::Result<Option<usize>> {
        if buf.len() >= BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN {
            // readings count is on last two bytes
            let readings: u16 = u16::from_le_bytes([
                buf[BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN - 2],
                buf[BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN - 1],
            ]);
            // how many bytes total before ASCII data
            let total =
                BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN + (readings as usize * READING_LEN);

            if buf.len() > total {
                if let Some(idx) = buf[total..].iter().position(|b| *b == b'\r') {
                    return Ok(Some(total + idx + EOL_LEN));
                }
            }
        }
        Ok(None) // Not enough data yet
    }
}

#[derive(Debug, Clone)]
pub struct RawSessionRecordReadings {
    pub start_ts: f64,
    pub end_ts: f64,
    pub span_readings: [RawReading; 3],
    pub sampling: u16,
    pub un2: u16,
    pub fixed_reading: RawReading,
    pub record_type: u16,
    pub stable: u16,
    pub transient_state: u16,
}

impl TryFrom<&[u8]> for RawSessionRecordReadings {
    type Error = std::io::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        assert!(value.len() >= BIN_MARKER_LEN + SAVED_RECORDING_METADATA_LEN);

        if value[0..2] == [b'#', b'0'] {
            let mut cur = Cursor::new(&value[2..]);

            let start_ts = read_double(&mut cur)?;
            let end_ts = read_double(&mut cur)?;

            let readings_cnt = 3;

            let mut readings = Vec::with_capacity(readings_cnt as usize);

            //assert_eq!(cur.remaining(), readings_cnt as usize * READING_LEN + 1);

            for _ in 0..readings_cnt {
                let mut buf = [0; READING_LEN];
                cur.read_exact(&mut buf)?;
                let reading = RawReading::try_from(&buf[..])?;
                readings.push(reading);
            }

            let sampling = cur.read_u16::<LittleEndian>()?;
            let un2 = cur.read_u16::<LittleEndian>()?;

            let mut buf = [0; READING_LEN];
            cur.read_exact(&mut buf)?;
            let reading2 = RawReading::try_from(&buf[..])?;

            let record_type = cur.read_u16::<LittleEndian>()?;

            let stable = cur.read_u16::<LittleEndian>()?;
            let transient_state = cur.read_u16::<LittleEndian>()?;

            Ok(RawSessionRecordReadings {
                start_ts,
                end_ts,
                span_readings: readings.try_into().map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "readings must contain 3 readings",
                    )
                })?,
                sampling,
                un2,
                fixed_reading: reading2,
                record_type,
                stable,
                transient_state,
            })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Binary data expected but not #0 marker found",
            ))
        }
    }
}

impl RawSessionRecordReadings {
    pub fn can_parse(buf: &[u8]) -> std::io::Result<Option<usize>> {
        //const STATUS_LEN: usize = 2;
        const EOL_LEN: usize = 1;

        let total = BIN_MARKER_LEN + SAVED_RECORD_READINGS_LEN;

        assert_eq!(total + EOL_LEN, 149); // QSRR returns fixed length

        if buf.len() >= total + EOL_LEN {
            return Ok(Some(total + EOL_LEN));
        }
        Ok(None) // Not enough data yet
    }
}
