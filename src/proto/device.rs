use futures::{SinkExt, StreamExt};
use std::time::SystemTime;
use std::{pin::Pin, time::Duration};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Decoder;

use super::Result;
use super::{
    codec::ProtocolCodec,
    command::Command,
    response::{Ident, Response, ResponsePayload},
    ProtoError,
};

trait AsyncReadWrite<S>: futures::Sink<S> + futures::Stream {}

impl<T, S> AsyncReadWrite<S> for T where T: futures::Sink<S> + futures::Stream {}

pub struct Device {
    //stream: Framed<SerialStream, ProtocolCodec>,
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
    pub fn new(com: String, baudrate: u32) -> Result<Self> {
        let mut port = tokio_serial::new(com, baudrate).open_native_async()?;

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
        let stream = ProtocolCodec::default().framed(super::fake::FakeBuffer::new(converted));

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
            Some(Ok(Response::Success(Some(ResponsePayload::Operator(company))))) => Ok(company),
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

    pub async fn clock(&mut self) -> Result<u64> {
        self.stream.send(Command::GetClock).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(Some(ResponsePayload::Clock(clock))))) => Ok(clock),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }

    pub async fn set_clock(&mut self, clock: SystemTime) -> Result<()> {
        let secs = clock
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_e| ProtoError::Abort)?
            .as_secs();

        self.stream.send(Command::SetClock(secs)).await?;
        match self.stream.next().await {
            Some(Ok(Response::Success(None))) => Ok(()),
            Some(Ok(response)) => Err(response.into()),
            Some(Err(ioerr)) => Err(ioerr.into()),
            None => Err(ProtoError::Abort),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

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
}
