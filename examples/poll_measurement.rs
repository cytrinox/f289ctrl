use f289ctrl::{measurement::Measurement, Device, DEFAULT_BAUDRATE};

#[tokio::main]
async fn main() -> f289ctrl::Result<()> {
    let path = "/dev/ttyUSB0".to_string();
    let mut device = Device::new(&path, DEFAULT_BAUDRATE)?;

    // Read device specific maps, required to convert RawMeasurement to Measurement.
    let maps = device.value_maps().await?;

    loop {
        let raw = device.live_measurement().await?;
        match raw {
            Some(data) => {
                let mea = Measurement::from((data, &maps));
                // Each measurement contains one or more readings.
                mea.readings.iter().for_each(|r| {
                    println!("Value: {}", r);
                })
            }
            None => {
                println!("NO_DATA");
            }
        }
    }
}
