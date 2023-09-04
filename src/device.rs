use chrono::{DateTime, Local, TimeZone, Utc};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::{pin::Pin, time::Duration};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Decoder;

use super::measurement::{Memory, SavedPeakMeasurement};
use super::proto::{
    codec::ProtocolCodec,
    command::Command,
    response::{Ident, Response, ResponsePayload},
    ProtoError,
};
use super::rawmea::{
    RawMeasurement, RawSavedMeasurement, RawSavedMinMaxMeasurement, RawSavedPeakMeasurement,
    RawSavedRecordingSessionInfo, RawSessionRecordReadings,
};
use crate::measurement::{SavedMeasurement, SavedMinMaxMeasurement, SavedRecordingSessionInfo};
use crate::proto::command::{
    ClearMemory, DateFormat, DezibelReference, DigitCount, Language, NumericFormat, TimeFormat,
};
use crate::proto::response::MemoryStat;
use crate::proto::Result;

trait AsyncReadWrite<S>: futures::Sink<S> + futures::Stream {}

impl<T, S> AsyncReadWrite<S> for T where T: futures::Sink<S> + futures::Stream {}

pub type ValueMap = HashMap<u16, String>;
pub type ValueMaps = HashMap<String, ValueMap>;

#[allow(clippy::type_complexity)]
pub struct Device {
    stream: Pin<
        Box<
            dyn AsyncReadWrite<
                Command,
                Error = std::io::Error,
                Item = std::result::Result<Response, std::io::Error>,
            >,
        >,
    >,
}

impl Device {
    pub fn new(com: impl AsRef<str>, baudrate: u32) -> Result<Self> {
        let mut port = tokio_serial::new(com.as_ref(), baudrate).open_native_async()?;

        #[cfg(unix)]
        port.set_exclusive(false)
            .expect("Unable to set serial port exclusive to false");

        let stream = ProtocolCodec::default().framed(port);

        Ok(Self {
            stream: Box::pin(stream),
        })
    }

    #[cfg(test)]
    pub fn new_faked(response_buf: Vec<char>) -> Self {
        let converted = response_buf.iter().map(|x| *x as u8).collect();
        let stream =
            ProtocolCodec::default().framed(super::proto::fake::FakeBuffer::new(converted));

        Self {
            stream: Box::pin(stream),
        }
    }

    pub async fn ident(&mut self) -> Result<Ident> {
        self.stream.send(Command::Id).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Id(id))))) => Ok(id),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn value_maps(&mut self) -> Result<ValueMaps> {
        let map_keys = [
            "primfunction",
            "secfunction",
            "autorange",
            "unit",
            "bolt",
            "mode",
            "state",
            "attribute",
            "recordtype",
            "isstableflag",
            "transientstate",
        ];

        let mut maps = ValueMaps::new();

        for k in &map_keys {
            self.stream
                .send(Command::QueryMap(String::from(*k)))
                .await?;
            match self.stream.next().await {
                Some(Ok(Response::Success(Some(ResponsePayload::Map(map))))) => {
                    maps.insert(k.to_string(), map);
                }
                Some(Ok(response)) => return Err(response.into()),
                Some(Err(ioerr)) => return Err(ioerr.into()),
                None => return Err(ProtoError::Abort),
            }
        }
        Ok(maps)
    }

    pub async fn all_memory(&mut self, maps: &ValueMaps) -> Result<Vec<Memory>> {
        let mea: Vec<SavedMeasurement> = self
            .saved_measurements_all()
            .await?
            .into_iter()
            .map(|raw| (raw, maps).into())
            .collect();

        let mea_minmax: Vec<SavedMinMaxMeasurement> = self
            .saved_minmax_all()
            .await?
            .into_iter()
            .map(|raw| (raw, maps).into())
            .collect();

        let mea_peak: Vec<SavedPeakMeasurement> = self
            .saved_peak_all()
            .await?
            .into_iter()
            .map(|raw| (raw, maps).into())
            .collect();

        let recordings: Vec<SavedRecordingSessionInfo> = self
            .saved_recordings_all()
            .await?
            .into_iter()
            .map(|raw| (raw, maps).into())
            .collect();

        Ok(mea
            .into_iter()
            .map(Memory::Measurement)
            .chain(mea_minmax.into_iter().map(Memory::MinMaxMeasurement))
            .chain(mea_peak.into_iter().map(Memory::PeakMeasurement))
            .chain(recordings.into_iter().map(Memory::Recording))
            .collect())
    }

    pub async fn backlight(&mut self) -> Result<Duration> {
        self.stream.send(Command::GetBacklightTimeout).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::BacklightTimeout(duration))))) => {
                Ok(duration)
            }
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_backlight(&mut self, duration: Duration) -> Result<()> {
        self.stream
            .send(Command::SetBacklightTimeout(duration))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn poweroff(&mut self) -> Result<Duration> {
        self.stream.send(Command::GetDevicePowerOff).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::DevicePowerOff(duration))))) => {
                Ok(duration)
            }
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_poweroff(&mut self, duration: Duration) -> Result<()> {
        self.stream
            .send(Command::SetDevicePowerOff(duration))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn operator(&mut self) -> Result<String> {
        self.stream.send(Command::GetOperator).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Operator(operator))))) => Ok(operator),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_operator(&mut self, operator: impl AsRef<str>) -> Result<()> {
        self.stream
            .send(Command::SetOperator(operator.as_ref().to_string()))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn company(&mut self) -> Result<String> {
        self.stream.send(Command::GetCompany).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Company(company))))) => Ok(company),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_company(&mut self, company: impl AsRef<str>) -> Result<()> {
        self.stream
            .send(Command::SetCompany(company.as_ref().to_string()))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn site(&mut self) -> Result<String> {
        self.stream.send(Command::GetSite).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Site(site))))) => Ok(site),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_site(&mut self, site: impl AsRef<str>) -> Result<()> {
        self.stream
            .send(Command::SetSite(site.as_ref().to_string()))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn contact(&mut self) -> Result<String> {
        self.stream.send(Command::GetContact).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Contact(contact))))) => Ok(contact),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_contact(&mut self, contact: impl AsRef<str>) -> Result<()> {
        self.stream
            .send(Command::SetContact(contact.as_ref().to_string()))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn beeper(&mut self) -> Result<bool> {
        self.stream.send(Command::GetBeeper).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Beeper(state))))) => Ok(state),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_beeper(&mut self, state: bool) -> Result<()> {
        self.stream.send(Command::SetBeeper(state)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn smoothing(&mut self) -> Result<bool> {
        self.stream.send(Command::GetSmoothing).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Smoothing(state))))) => Ok(state),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_smoothing(&mut self, state: bool) -> Result<()> {
        self.stream.send(Command::SetSmoothing(state)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn clock(&mut self) -> Result<u64> {
        self.stream.send(Command::GetClock).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Clock(clock))))) => Ok(clock),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_clock(&mut self, clock: DateTime<Local>) -> Result<()> {
        let naive = clock.naive_local();
        let utc: DateTime<Utc> = Utc.from_utc_datetime(&naive);
        let secs = utc.timestamp() as u64;

        /*
        let secs = clock
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_e| ProtoError::Abort)?
            .as_secs();
             */

        self.stream.send(Command::SetClock(secs)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn clear(&mut self, mem: ClearMemory) -> Result<()> {
        self.stream.send(Command::Clear(mem)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.stream.send(Command::ResetDevice).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn custom_dbm(&mut self) -> Result<u16> {
        self.stream.send(Command::GetCustomDbm).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::CustomDbm(dbm))))) => Ok(dbm),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_custom_dbm(&mut self, dbm: u16) -> Result<()> {
        self.stream.send(Command::SetCustomDbm(dbm)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn dbm_ref(&mut self) -> Result<DezibelReference> {
        self.stream.send(Command::GetDbmRef).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::DbmRef(dbm))))) => Ok(dbm),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_dbm_ref(&mut self, dbm: DezibelReference) -> Result<()> {
        self.stream.send(Command::SetDbmRef(dbm)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn temp_offset(&mut self) -> Result<i16> {
        self.stream.send(Command::GetTempOffset).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::TempOffset(offset))))) => Ok(offset),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_temp_offset(&mut self, offset: i16) -> Result<()> {
        self.stream.send(Command::SetTempOffset(offset)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn digit_count(&mut self) -> Result<DigitCount> {
        self.stream.send(Command::GetDigitCount).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::DigitCount(dc))))) => Ok(dc),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_digit_count(&mut self, dc: DigitCount) -> Result<()> {
        self.stream.send(Command::SetDigitCount(dc)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn autohold_event_threshold(&mut self) -> Result<u8> {
        self.stream.send(Command::GetAutoHoldEventThreshold).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::AutoHoldEventThreshold(thd))))) => {
                Ok(thd)
            }
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_autohold_event_threshold(&mut self, thd: u8) -> Result<()> {
        self.stream
            .send(Command::SetAutoHoldEventThreshold(thd))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn recording_event_threshold(&mut self) -> Result<u8> {
        self.stream
            .send(Command::GetRecordingEventThreshold)
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::RecordingEventThreshold(thd))))) => {
                Ok(thd)
            }
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_recording_event_threshold(&mut self, thd: u8) -> Result<()> {
        self.stream
            .send(Command::SetRecordingEventThreshold(thd))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn language(&mut self) -> Result<Language> {
        self.stream.send(Command::GetLanguage).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Language(lang))))) => Ok(lang),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_language(&mut self, lang: Language) -> Result<()> {
        self.stream.send(Command::SetLanguage(lang)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn date_format(&mut self) -> Result<DateFormat> {
        self.stream.send(Command::GetDateFormat).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::DateFormat(fmt))))) => Ok(fmt),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_date_format(&mut self, fmt: DateFormat) -> Result<()> {
        self.stream.send(Command::SetDateFormat(fmt)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn time_format(&mut self) -> Result<TimeFormat> {
        self.stream.send(Command::GetTimeFormat).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::TimeFormat(fmt))))) => Ok(fmt),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_time_format(&mut self, fmt: TimeFormat) -> Result<()> {
        self.stream.send(Command::SetTimeFormat(fmt)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn numeric_format(&mut self) -> Result<NumericFormat> {
        self.stream.send(Command::GetNumFormat).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::NumericFormat(fmt))))) => Ok(fmt),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_numeric_format(&mut self, fmt: NumericFormat) -> Result<()> {
        self.stream.send(Command::SetNumFormat(fmt)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn save_name(&mut self, slot: u16) -> Result<String> {
        self.stream.send(Command::GetSaveName(slot)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::SaveName(name))))) => Ok(name),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_save_name(&mut self, slot: u16, name: impl AsRef<str>) -> Result<()> {
        self.stream
            .send(Command::SetSaveName(slot, name.as_ref().to_string()))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn live_measurement(&mut self) -> Result<Option<RawMeasurement>> {
        self.stream.send(Command::GetMeasurementBinary).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::MeasurementBinary(m))))) => Ok(Some(m)),
            Some(Ok(Response::NoData)) => Ok(None),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn memory_statistics(&mut self) -> Result<MemoryStat> {
        self.stream.send(Command::GetMemoryStat).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::MemoryStat(m))))) => Ok(m),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn saved_measurement(&mut self, idx: usize) -> Result<RawSavedMeasurement> {
        self.stream
            .send(Command::QuerySavedMeasurement(idx))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::SavedMeasurement(m))))) => Ok(m),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn saved_measurements_all(&mut self) -> Result<Vec<RawSavedMeasurement>> {
        let stats = self.memory_statistics().await?;
        let mut v = Vec::with_capacity(stats.measurement);
        for i in 0..stats.measurement {
            let m = self.saved_measurement(i).await?;
            v.push(m);
        }
        Ok(v)
    }

    pub async fn saved_minmax(&mut self, idx: usize) -> Result<RawSavedMinMaxMeasurement> {
        self.stream
            .send(Command::QueryMinMaxSessionInfo(idx))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::MinMaxSessionInfo(m))))) => Ok(m),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn saved_minmax_all(&mut self) -> Result<Vec<RawSavedMinMaxMeasurement>> {
        let stats = self.memory_statistics().await?;
        let mut v = Vec::with_capacity(stats.min_max);
        for i in 0..stats.min_max {
            let m = self.saved_minmax(i).await?;
            v.push(m);
        }
        Ok(v)
    }

    pub async fn saved_peak(&mut self, idx: usize) -> Result<RawSavedPeakMeasurement> {
        self.stream.send(Command::QueryPeakSessionInfo(idx)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::PeakSessionInfo(m))))) => Ok(m),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn saved_peak_all(&mut self) -> Result<Vec<RawSavedPeakMeasurement>> {
        let stats = self.memory_statistics().await?;
        let mut v = Vec::with_capacity(stats.peak);
        for i in 0..stats.peak {
            let m = self.saved_peak(i).await?;
            v.push(m);
        }
        Ok(v)
    }

    pub async fn saved_recording(&mut self, idx: usize) -> Result<RawSavedRecordingSessionInfo> {
        self.stream
            .send(Command::QueryRecordedSessionInfo(idx))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::RecordedSessionInfo(m))))) => Ok(m),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn saved_recordings_all(&mut self) -> Result<Vec<RawSavedRecordingSessionInfo>> {
        let stats = self.memory_statistics().await?;
        let mut v = Vec::with_capacity(stats.recordings);
        for i in 0..stats.recordings {
            let m = self.saved_recording(i).await?;
            v.push(m);
        }
        Ok(v)
    }

    pub async fn session_record_reading(
        &mut self,
        reading_idx: usize,
        sample_idx: usize,
    ) -> Result<RawSessionRecordReadings> {
        self.stream
            .send(Command::QuerySessionRecordReadings(reading_idx, sample_idx))
            .await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::SessionRecordReading(m))))) => Ok(m),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn session_record_reading_all_cb(
        &mut self,
        reading_index: usize,
        num_samples: usize,
        callback: impl FnOnce(usize, usize) + Copy + 'static,
    ) -> Result<Vec<RawSessionRecordReadings>> {
        let mut v = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let m = self.session_record_reading(reading_index, i).await?;
            callback(i, num_samples);

            v.push(m);
        }
        Ok(v)
    }

    pub async fn session_record_reading_all(
        &mut self,
        reading_index: usize,
        num_samples: usize,
    ) -> Result<Vec<RawSessionRecordReadings>> {
        self.session_record_reading_all_cb(reading_index, num_samples, |_, _| {})
            .await
    }
}

#[cfg(test)]
mod tests {

    use crate::measurement::{Measurement, Reading};

    use super::*;

    const GETEMAP: [u8; 1452] = [
        0x30, 0x0d, 0x34, 0x39, 0x2c, 0x30, 0x2c, 0x4c, 0x49, 0x4d, 0x42, 0x4f, 0x2c, 0x31, 0x2c,
        0x56, // 0.49,0,LIMBO,1,V
        0x5f, 0x41, 0x43, 0x2c, 0x32, 0x2c, 0x4d, 0x56, 0x5f, 0x41, 0x43, 0x2c, 0x33, 0x2c, 0x56,
        0x5f, // _AC,2,MV_AC,3,V_
        0x44, 0x43, 0x2c, 0x34, 0x2c, 0x4d, 0x56, 0x5f, 0x44, 0x43, 0x2c, 0x35, 0x2c, 0x56, 0x5f,
        0x41, //0xDC,,4,MV_DC,5,V_A
        0x43, 0x5f, 0x4f, 0x56, 0x45, 0x52, 0x5f, 0x44, 0x43, 0x2c, 0x36, 0x2c, 0x56, 0x5f, 0x44,
        0x43, // C_OVER_DC,6,V_DC
        0x5f, 0x4f, 0x56, 0x45, 0x52, 0x5f, 0x41, 0x43, 0x2c, 0x37, 0x2c, 0x56, 0x5f, 0x41, 0x43,
        0x5f, // _OVER_AC,7,V_AC_
        0x50, 0x4c, 0x55, 0x53, 0x5f, 0x44, 0x43, 0x2c, 0x38, 0x2c, 0x4d, 0x56, 0x5f, 0x41, 0x43,
        0x5f, // PLUS_DC,8,MV_AC_
        0x4f, 0x56, 0x45, 0x52, 0x5f, 0x44, 0x43, 0x2c, 0x39, 0x2c, 0x4d, 0x56, 0x5f, 0x44, 0x43,
        0x5f, // OVER_DC,9,MV_DC_
        0x4f, 0x56, 0x45, 0x52, 0x5f, 0x41, 0x43, 0x2c, 0x31, 0x30, 0x2c, 0x4d, 0x56, 0x5f, 0x41,
        0x43, // OVER_AC,10,MV_AC
        0x5f, 0x50, 0x4c, 0x55, 0x53, 0x5f, 0x44, 0x43, 0x2c, 0x31, 0x31, 0x2c, 0x41, 0x5f, 0x41,
        0x43, // _PLUS_DC,11,A_AC
        0x2c, 0x31, 0x32, 0x2c, 0x4d, 0x41, 0x5f, 0x41, 0x43, 0x2c, 0x31, 0x33, 0x2c, 0x55, 0x41,
        0x5f, // ,12,MA_AC,13,UA_
        0x41, 0x43, 0x2c, 0x31, 0x34, 0x2c, 0x41, 0x5f, 0x44, 0x43, 0x2c, 0x31, 0x35, 0x2c, 0x4d,
        0x41, //0xAC,,14,A_DC,15,MA
        0x5f, 0x44, 0x43, 0x2c, 0x31, 0x36, 0x2c, 0x55, 0x41, 0x5f, 0x44, 0x43, 0x2c, 0x31, 0x37,
        0x2c, // _DC,16,UA_DC,17,
        0x41, 0x5f, 0x41, 0x43, 0x5f, 0x4f, 0x56, 0x45, 0x52, 0x5f, 0x44, 0x43, 0x2c, 0x31, 0x38,
        0x2c, // A_AC_OVER_DC,18,
        0x41, 0x5f, 0x44, 0x43, 0x5f, 0x4f, 0x56, 0x45, 0x52, 0x5f, 0x41, 0x43, 0x2c, 0x31, 0x39,
        0x2c, // A_DC_OVER_AC,19,
        0x41, 0x5f, 0x41, 0x43, 0x5f, 0x50, 0x4c, 0x55, 0x53, 0x5f, 0x44, 0x43, 0x2c, 0x32, 0x30,
        0x2c, // A_AC_PLUS_DC,20,
        0x4d, 0x41, 0x5f, 0x41, 0x43, 0x5f, 0x4f, 0x56, 0x45, 0x52, 0x5f, 0x44, 0x43, 0x2c, 0x32,
        0x31, // MA_AC_OVER_DC,21
        0x2c, 0x4d, 0x41, 0x5f, 0x44, 0x43, 0x5f, 0x4f, 0x56, 0x45, 0x52, 0x5f, 0x41, 0x43, 0x2c,
        0x32, // ,MA_DC_OVER_AC,2
        0x32, 0x2c, 0x4d, 0x41, 0x5f, 0x41, 0x43, 0x5f, 0x50, 0x4c, 0x55, 0x53, 0x5f, 0x44, 0x43,
        0x2c, // 2,MA_AC_PLUS_DC,
        0x32, 0x33, 0x2c, 0x55, 0x41, 0x5f, 0x41, 0x43, 0x5f, 0x4f, 0x56, 0x45, 0x52, 0x5f, 0x44,
        0x43, //0x23,,UA_AC_OVER_DC
        0x2c, 0x32, 0x34, 0x2c, 0x55, 0x41, 0x5f, 0x44, 0x43, 0x5f, 0x4f, 0x56, 0x45, 0x52, 0x5f,
        0x41, // ,24,UA_DC_OVER_A
        0x43, 0x2c, 0x32, 0x35, 0x2c, 0x55, 0x41, 0x5f, 0x41, 0x43, 0x5f, 0x50, 0x4c, 0x55, 0x53,
        0x5f, // C,25,UA_AC_PLUS_
        0x44, 0x43, 0x2c, 0x32, 0x36, 0x2c, 0x54, 0x45, 0x4d, 0x50, 0x45, 0x52, 0x41, 0x54, 0x55,
        0x52, //0xDC,,26,TEMPERATUR
        0x45, 0x2c, 0x32, 0x37, 0x2c, 0x4f, 0x48, 0x4d, 0x53, 0x2c, 0x32, 0x38, 0x2c, 0x43, 0x4f,
        0x4e, // E,27,OHMS,28,CON
        0x44, 0x55, 0x43, 0x54, 0x41, 0x4e, 0x43, 0x45, 0x2c, 0x32, 0x39, 0x2c, 0x43, 0x4f, 0x4e,
        0x54, // DUCTANCE,29,CONT
        0x49, 0x4e, 0x55, 0x49, 0x54, 0x59, 0x2c, 0x33, 0x30, 0x2c, 0x43, 0x41, 0x50, 0x41, 0x43,
        0x49, // INUITY,30,CAPACI
        0x54, 0x41, 0x4e, 0x43, 0x45, 0x2c, 0x33, 0x31, 0x2c, 0x44, 0x49, 0x4f, 0x44, 0x45, 0x5f,
        0x54, // TANCE,31,DIODE_T
        0x45, 0x53, 0x54, 0x2c, 0x33, 0x32, 0x2c, 0x56, 0x5f, 0x41, 0x43, 0x5f, 0x4c, 0x4f, 0x5a,
        0x2c, // EST,32,V_AC_LOZ,
        0x33, 0x33, 0x2c, 0x4f, 0x48, 0x4d, 0x53, 0x5f, 0x4c, 0x4f, 0x57, 0x2c, 0x33, 0x34, 0x2c,
        0x43, //0x33,,OHMS_LOW,34,C
        0x41, 0x4c, 0x5f, 0x56, 0x5f, 0x44, 0x43, 0x5f, 0x4c, 0x4f, 0x5a, 0x2c, 0x33, 0x35, 0x2c,
        0x43, // AL_V_DC_LOZ,35,C
        0x41, 0x4c, 0x5f, 0x41, 0x44, 0x5f, 0x47, 0x41, 0x49, 0x4e, 0x5f, 0x58, 0x32, 0x2c, 0x33,
        0x36, // AL_AD_GAIN_X2,36
        0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x41, 0x44, 0x5f, 0x47, 0x41, 0x49, 0x4e, 0x5f, 0x58, 0x31,
        0x2c, // ,CAL_AD_GAIN_X1,
        0x33, 0x37, 0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x52, 0x4d, 0x53, 0x2c, 0x33, 0x38, 0x2c, 0x43,
        0x41, //0x37,,CAL_RMS,38,CA
        0x4c, 0x5f, 0x46, 0x49, 0x4c, 0x54, 0x5f, 0x41, 0x4d, 0x50, 0x2c, 0x33, 0x39, 0x2c, 0x43,
        0x41, // L_FILT_AMP,39,CA
        0x4c, 0x5f, 0x44, 0x43, 0x5f, 0x41, 0x4d, 0x50, 0x5f, 0x58, 0x35, 0x2c, 0x34, 0x30, 0x2c,
        0x43, // L_DC_AMP_X5,40,C
        0x41, 0x4c, 0x5f, 0x44, 0x43, 0x5f, 0x41, 0x4d, 0x50, 0x5f, 0x58, 0x31, 0x30, 0x2c, 0x34,
        0x31, // AL_DC_AMP_X10,41
        0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x4e, 0x49, 0x4e, 0x56, 0x5f, 0x41, 0x43, 0x5f, 0x41, 0x4d,
        0x50, // ,CAL_NINV_AC_AMP
        0x2c, 0x34, 0x32, 0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x49, 0x53, 0x52, 0x43, 0x5f, 0x35, 0x30,
        0x30, // ,42,CAL_ISRC_500
        0x4e, 0x41, 0x2c, 0x34, 0x33, 0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x43, 0x4f, 0x4d, 0x50, 0x5f,
        0x54, // NA,43,CAL_COMP_T
        0x52, 0x49, 0x4d, 0x5f, 0x4d, 0x56, 0x5f, 0x44, 0x43, 0x2c, 0x34, 0x34, 0x2c, 0x43, 0x41,
        0x4c, // RIM_MV_DC,44,CAL
        0x5f, 0x41, 0x43, 0x44, 0x43, 0x5f, 0x41, 0x43, 0x5f, 0x43, 0x4f, 0x4d, 0x50, 0x2c, 0x34,
        0x35, // _ACDC_AC_COMP,45
        0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x56, 0x5f, 0x41, 0x43, 0x5f, 0x4c, 0x4f, 0x5a, 0x2c, 0x34,
        0x36, // ,CAL_V_AC_LOZ,46
        0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x56, 0x5f, 0x41, 0x43, 0x5f, 0x50, 0x45, 0x41, 0x4b, 0x2c,
        0x34, // ,CAL_V_AC_PEAK,4
        0x37, 0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x4d, 0x56, 0x5f, 0x41, 0x43, 0x5f, 0x50, 0x45, 0x41,
        0x4b, // 7,CAL_MV_AC_PEAK
        0x2c, 0x34, 0x38, 0x2c, 0x43, 0x41, 0x4c, 0x5f, 0x54, 0x45, 0x4d, 0x50, 0x45, 0x52, 0x41,
        0x54, // ,48,CAL_TEMPERAT
        0x55, 0x52, 0x45, 0x0d, //             URE.
        //
        0x30, 0x0d, 0x31, 0x30, 0x2c, 0x30, 0x2c, 0x4e, 0x4f, 0x4e, 0x45, 0x2c, 0x31, 0x2c, 0x48,
        0x45, // 0.10,0,NONE,1,HE
        0x52, 0x54, 0x5a, 0x2c, 0x32, 0x2c, 0x44, 0x55, 0x54, 0x59, 0x5f, 0x43, 0x59, 0x43, 0x4c,
        0x45, // RTZ,2,DUTY_CYCLE
        0x2c, 0x33, 0x2c, 0x50, 0x55, 0x4c, 0x53, 0x45, 0x5f, 0x57, 0x49, 0x44, 0x54, 0x48, 0x2c,
        0x34, // ,3,PULSE_WIDTH,4
        0x2c, 0x44, 0x42, 0x4d, 0x2c, 0x35, 0x2c, 0x44, 0x42, 0x56, 0x2c, 0x36, 0x2c, 0x44, 0x42,
        0x4d, // ,DBM,5,DBV,6,DBM
        0x5f, 0x48, 0x45, 0x52, 0x54, 0x5a, 0x2c, 0x37, 0x2c, 0x44, 0x42, 0x56, 0x5f, 0x48, 0x45,
        0x52, // _HERTZ,7,DBV_HER
        0x54, 0x5a, 0x2c, 0x38, 0x2c, 0x43, 0x52, 0x45, 0x53, 0x54, 0x5f, 0x46, 0x41, 0x43, 0x54,
        0x4f, // TZ,8,CREST_FACTO
        0x52, 0x2c, 0x39, 0x2c, 0x50, 0x45, 0x41, 0x4b, 0x5f, 0x4d, 0x49, 0x4e, 0x5f, 0x4d, 0x41,
        0x58, // R,9,PEAK_MIN_MAX
        0x0d, //                .
        0x30, 0x0d, 0x32, 0x2c, 0x31, 0x2c, 0x41, 0x55, 0x54, 0x4f, 0x2c, 0x30, 0x2c, 0x4d, 0x41,
        0x4e, // 0.2,1,AUTO,0,MAN
        0x55, 0x41, 0x4c, 0x0d, //             UAL.
        0x30, 0x0d, 0x32, 0x31, 0x2c, 0x30, 0x2c, 0x4e, 0x4f, 0x4e, 0x45, 0x2c, 0x31, 0x2c, 0x56,
        0x44, // 0.21,0,NONE,1,VD
        0x43, 0x2c, 0x32, 0x2c, 0x56, 0x41, 0x43, 0x2c, 0x33, 0x2c, 0x56, 0x41, 0x43, 0x5f, 0x50,
        0x4c, // C,2,VAC,3,VAC_PL
        0x55, 0x53, 0x5f, 0x44, 0x43, 0x2c, 0x34, 0x2c, 0x56, 0x2c, 0x35, 0x2c, 0x41, 0x44, 0x43,
        0x2c, // US_DC,4,V,5,ADC,
        0x36, 0x2c, 0x41, 0x41, 0x43, 0x2c, 0x37, 0x2c, 0x41, 0x41, 0x43, 0x5f, 0x50, 0x4c, 0x55,
        0x53, // 6,AAC,7,AAC_PLUS
        0x5f, 0x44, 0x43, 0x2c, 0x38, 0x2c, 0x41, 0x2c, 0x39, 0x2c, 0x4f, 0x48, 0x4d, 0x2c, 0x31,
        0x30, // _DC,8,A,9,OHM,10
        0x2c, 0x53, 0x49, 0x45, 0x2c, 0x31, 0x31, 0x2c, 0x48, 0x7a, 0x2c, 0x31, 0x32, 0x2c, 0x53,
        0x2c, // ,SIE,11,Hz,12,S,
        0x31, 0x33, 0x2c, 0x46, 0x2c, 0x31, 0x34, 0x2c, 0x43, 0x45, 0x4c, 0x2c, 0x31, 0x35, 0x2c,
        0x46, //0x13,,F,14,CEL,15,F
        0x41, 0x52, 0x2c, 0x31, 0x36, 0x2c, 0x50, 0x43, 0x54, 0x2c, 0x31, 0x37, 0x2c, 0x64, 0x42,
        0x2c, // AR,16,PCT,17,dB,
        0x31, 0x38, 0x2c, 0x64, 0x42, 0x56, 0x2c, 0x31, 0x39, 0x2c, 0x64, 0x42, 0x6d, 0x2c, 0x32,
        0x30, //0x18,,dBV,19,dBm,20
        0x2c, 0x43, 0x52, 0x45, 0x53, 0x54, 0x5f, 0x46, 0x41, 0x43, 0x54, 0x4f, 0x52,
        0x0d, //   ,CREST_FACTOR.
        0x30, 0x0d, 0x32, 0x2c, 0x30, 0x2c, 0x4f, 0x46, 0x46, 0x2c, 0x31, 0x2c, 0x4f, 0x4e,
        0x0d, //  0.2,0,OFF,1,ON.
        0x30, 0x0d, 0x31, 0x30, 0x2c, 0x30, 0x2c, 0x4e, 0x4f, 0x4e, 0x45, 0x2c, 0x31, 0x2c, 0x41,
        0x55, // 0.10,0,NONE,1,AU
        0x54, 0x4f, 0x5f, 0x48, 0x4f, 0x4c, 0x44, 0x2c, 0x32, 0x2c, 0x41, 0x55, 0x54, 0x4f, 0x5f,
        0x53, // TO_HOLD,2,AUTO_S
        0x41, 0x56, 0x45, 0x2c, 0x34, 0x2c, 0x48, 0x4f, 0x4c, 0x44, 0x2c, 0x38, 0x2c, 0x4c, 0x4f,
        0x57, // AVE,4,HOLD,8,LOW
        0x5f, 0x50, 0x41, 0x53, 0x53, 0x5f, 0x46, 0x49, 0x4c, 0x54, 0x45, 0x52, 0x2c, 0x31, 0x36,
        0x2c, // _PASS_FILTER,16,
        0x4d, 0x49, 0x4e, 0x5f, 0x4d, 0x41, 0x58, 0x5f, 0x41, 0x56, 0x47, 0x2c, 0x33, 0x32, 0x2c,
        0x52, // MIN_MAX_AVG,32,R
        0x45, 0x43, 0x4f, 0x52, 0x44, 0x2c, 0x36, 0x34, 0x2c, 0x52, 0x45, 0x4c, 0x2c, 0x31, 0x32,
        0x38, //0xEC,ORD,64,REL,128
        0x2c, 0x52, 0x45, 0x4c, 0x5f, 0x50, 0x45, 0x52, 0x43, 0x45, 0x4e, 0x54, 0x2c, 0x32, 0x35,
        0x36, // ,REL_PERCENT,256
        0x2c, 0x43, 0x41, 0x4c, 0x49, 0x42, 0x52, 0x41, 0x54, 0x49, 0x4f, 0x4e,
        0x0d, //    ,CALIBRATION.
        0x30, 0x0d, 0x38, 0x2c, 0x30, 0x2c, 0x49, 0x4e, 0x41, 0x43, 0x54, 0x49, 0x56, 0x45, 0x2c,
        0x31, // 0.8,0,INACTIVE,1
        0x2c, 0x49, 0x4e, 0x56, 0x41, 0x4c, 0x49, 0x44, 0x2c, 0x32, 0x2c, 0x4e, 0x4f, 0x52, 0x4d,
        0x41, // ,INVALID,2,NORMA
        0x4c, 0x2c, 0x33, 0x2c, 0x42, 0x4c, 0x41, 0x4e, 0x4b, 0x2c, 0x34, 0x2c, 0x44, 0x49, 0x53,
        0x43, // L,3,BLANK,4,DISC
        0x48, 0x41, 0x52, 0x47, 0x45, 0x2c, 0x35, 0x2c, 0x4f, 0x4c, 0x2c, 0x36, 0x2c, 0x4f, 0x4c,
        0x5f, // HARGE,5,OL,6,OL_
        0x4d, 0x49, 0x4e, 0x55, 0x53, 0x2c, 0x37, 0x2c, 0x4f, 0x50, 0x45, 0x4e, 0x5f, 0x54, 0x43,
        0x0d, // MINUS,7,OPEN_TC.
        0x30, 0x0d, 0x39, 0x2c, 0x30, 0x2c, 0x4e, 0x4f, 0x4e, 0x45, 0x2c, 0x31, 0x2c, 0x4f, 0x50,
        0x45, // 0.9,0,NONE,1,OPE
        0x4e, 0x5f, 0x43, 0x49, 0x52, 0x43, 0x55, 0x49, 0x54, 0x2c, 0x32, 0x2c, 0x53, 0x48, 0x4f,
        0x52, // N_CIRCUIT,2,SHOR
        0x54, 0x5f, 0x43, 0x49, 0x52, 0x43, 0x55, 0x49, 0x54, 0x2c, 0x33, 0x2c, 0x47, 0x4c, 0x49,
        0x54, // T_CIRCUIT,3,GLIT
        0x43, 0x48, 0x5f, 0x43, 0x49, 0x52, 0x43, 0x55, 0x49, 0x54, 0x2c, 0x34, 0x2c, 0x47, 0x4f,
        0x4f, // CH_CIRCUIT,4,GOO
        0x44, 0x5f, 0x44, 0x49, 0x4f, 0x44, 0x45, 0x2c, 0x35, 0x2c, 0x4c, 0x4f, 0x5f, 0x4f, 0x48,
        0x4d, // D_DIODE,5,LO_OHM
        0x53, 0x2c, 0x36, 0x2c, 0x4e, 0x45, 0x47, 0x41, 0x54, 0x49, 0x56, 0x45, 0x5f, 0x45, 0x44,
        0x47, // S,6,NEGATIVE_EDG
        0x45, 0x2c, 0x37, 0x2c, 0x50, 0x4f, 0x53, 0x49, 0x54, 0x49, 0x56, 0x45, 0x5f, 0x45, 0x44,
        0x47, // E,7,POSITIVE_EDG
        0x45, 0x2c, 0x38, 0x2c, 0x48, 0x49, 0x47, 0x48, 0x5f, 0x43, 0x55, 0x52, 0x52, 0x45, 0x4e,
        0x54, // E,8,HIGH_CURRENT
        0x0d, //                .
        0x30, 0x0d, 0x32, 0x2c, 0x30, 0x2c, 0x49, 0x4e, 0x50, 0x55, 0x54, 0x2c, 0x31, 0x2c, 0x49,
        0x4e, // 0.2,0,INPUT,1,IN
        0x54, 0x45, 0x52, 0x56, 0x41, 0x4c, 0x0d, //          TERVAL.
        0x30, 0x0d, 0x32, 0x2c, 0x30, 0x2c, 0x55, 0x4e, 0x53, 0x54, 0x41, 0x42, 0x4c, 0x45, 0x2c,
        0x31, // 0.2,0,UNSTABLE,1
        0x2c, 0x53, 0x54, 0x41, 0x42, 0x4c, 0x45, 0x0d, //         ,STABLE.
        0x30, 0x0d, 0x35, 0x2c, 0x30, 0x2c, 0x4e, 0x4f, 0x4e, 0x5f, 0x54, 0x2c, 0x31, 0x2c, 0x52,
        0x41, // 0.5,0,NON_T,1,RA
        0x4e, 0x47, 0x45, 0x5f, 0x55, 0x50, 0x2c, 0x32, 0x2c, 0x52, 0x41, 0x4e, 0x47, 0x45, 0x5f,
        0x44, // NGE_UP,2,RANGE_D
        0x4f, 0x57, 0x4e, 0x2c, 0x33, 0x2c, 0x4f, 0x56, 0x45, 0x52, 0x4c, 0x4f, 0x41, 0x44, 0x2c,
        0x34, // OWN,3,OVERLOAD,4
        0x2c, 0x4f, 0x50, 0x45, 0x4e, 0x5f, 0x54, 0x43, 0x0d, //        ,OPEN_TC.
    ];

    #[tokio::test]
    async fn test_get_id() {
        let mut device = Device::new_faked(vec![
            '0', '\r', 'F', 'l', 'u', 'k', 'e', ',', 'x', ',', 'x', '\r',
        ]);
        assert!(device.ident().await.is_ok());
    }

    #[tokio::test]
    async fn test_set_backlight() {
        let mut device = Device::new_faked(vec!['0', '\r']);
        assert!(device
            .set_backlight(Duration::from_secs(60 * 15))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_set_backlight_in_settings_mode() {
        let mut device = Device::new_faked(vec!['2', '\r']);
        assert!(device
            .set_backlight(Duration::from_secs(60 * 15))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn qddb_parse() {
        let fake: Vec<u8> = vec![
            0x30, 0x0d, 0x23, 0x30, 0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x09, 0x00, 0x00, 0x40,
            0x7f, 0x40, // l1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // l2
            0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0xc2, 0xf5, 0x11, 0x40, 0xf6, 0x28,
            0x5c, 0x8f, // l3
            0x09, 0x00, 0x00, 0x00, 0x02, 0x00, 0x05, 0x00, 0x02, 0x00, 0x00, 0x00, 0xbf, 0xf3,
            0xd8, 0x41, // l4
            0x00, 0x40, 0x9d, 0xeb, 0x02, 0x00, 0xc2, 0xf5, 0x11, 0x40, 0xf6, 0x28, 0x5c, 0x8f,
            0x09, 0x00, // l5
            0x00, 0x00, 0x02, 0x00, 0x05, 0x00, 0x02, 0x00, 0x00, 0x00, 0xbf, 0xf3, 0xd8, 0x41,
            0x00, 0x40, // l6
            0x9d, 0xeb, 0x0d, // l7
        ];

        let mut device = Device::new_faked(
            GETEMAP
                .iter()
                .chain(fake.iter())
                .map(|x| *x as char)
                .collect(),
        );

        let maps = device.value_maps().await.expect("Value Maps");

        let raw_mea = device
            .live_measurement()
            .await
            .expect("Raw measurement")
            .expect("No data returned");
        println!("Raw measurement: {:?}", raw_mea);
        assert_eq!(raw_mea.pri_function, 27);
        assert_eq!(raw_mea.sec_function, 0);
        assert_eq!(raw_mea.auto_range, 1);
        assert_eq!(raw_mea.unit, 9);
        assert_eq!(raw_mea.unit_multiplier, 0);
        assert_eq!(raw_mea.bolt, 0);

        assert_eq!(raw_mea.bolt, 0);
        assert_eq!(raw_mea.modes, 0);
        assert_eq!(raw_mea.readings.len(), 2);

        println!("{:?}", Measurement::from((raw_mea.clone(), &maps)));

        for rr in &raw_mea.readings {
            let r: Reading = (rr.clone(), &maps).into();
            println!("{}", r);
        }

        // TODO: check readings
    }
}
