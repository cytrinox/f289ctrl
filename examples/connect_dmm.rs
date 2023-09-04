use f289ctrl::{Device, DEFAULT_BAUDRATE};

#[tokio::main]
async fn main() -> f289ctrl::Result<()> {
    let path = "/dev/ttyUSB0".to_string();
    let mut device = Device::new(&path, DEFAULT_BAUDRATE)?;
    eprintln!("Connected to: {}\n", device.ident().await?.model);
    Ok(())
}
