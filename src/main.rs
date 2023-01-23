#![deny(clippy::unwrap_used)]

use chrono::{DateTime, Local, Utc};
use clap::builder::BoolishValueParser;
use clap::{arg, command, value_parser};
use std::io::ErrorKind;
use std::process::exit;
use std::time::SystemTime;
use std::{env, path::PathBuf, str, time::Duration};

pub mod proto;

use crate::proto::device::Device;
use crate::proto::Result;

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyUSB0";
#[cfg(windows)]
const DEFAULT_TTY: &str = "COM1";

const DEFAULT_BAUDRATE: u32 = 115200;

#[tokio::main]
async fn main() -> tokio_serial::Result<()> {
    let matches = command!() // requires `cargo` feature
        .arg(
            arg!(
                -p --device <PORT> "Port for USB adapter"
            )
            .default_value(DEFAULT_TTY)
            .required(false)
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(arg!(
            -d --debug ... "Turn debugging information on"
        ))
        .arg(
            arg!(
                -b --baudrate <BAUDRATE> "Baudrate"
            )
            .default_value(DEFAULT_BAUDRATE.to_string())
            .value_parser(value_parser!(u32)),
        )
        .subcommand(
            clap::Command::new("backlight")
                .about("Auto Backlight Timeout")
                .arg(
                    arg!([minutes] "Set time in minutes for auto backlight timeout")
                        .value_parser(["5", "10", "15", "20", "25", "30", "off"]),
                ),
        )
        .subcommand(
            clap::Command::new("poweroff").about("Auto Power Off").arg(
                arg!([minutes] "Set time in minutes for auto power off")
                    .value_parser(["15", "25", "35", "45", "60", "off"]),
            ),
        )
        .subcommand(
            clap::Command::new("smoothing")
                .about("Smoothing (AC)")
                .arg(arg!([enabled] "Set smoothing").value_parser(BoolishValueParser::new())),
        )
        .subcommand(clap::Command::new("ident").about("Device identification"))
        .subcommand(
            clap::Command::new("beeper")
                .about("Beeper")
                .arg(arg!([enabled] "Set beeper").value_parser(BoolishValueParser::new())),
        )
        .subcommand(
            clap::Command::new("clock")
                .about("Internal clock")
                .arg(arg!(
                    --"sync-with-host" "Sync DMM clock with local host"
                )),
        )
        .subcommand(
            clap::Command::new("operator")
                .about("Operator name")
                .arg(arg!([name] "Set operator name")),
        )
        .subcommand(
            clap::Command::new("company")
                .about("Company name")
                .arg(arg!([name] "Set company name")),
        )
        .subcommand(
            clap::Command::new("site")
                .about("Site name")
                .arg(arg!([name] "Set site name")),
        )
        .subcommand(
            clap::Command::new("contact")
                .about("Contact")
                .arg(arg!([name] "Set contact")),
        )
        .subcommand_required(true)
        .get_matches();

    match handle_args(&matches).await {
        Ok(()) => {}
        Err(e) => {
            //eprintln!("{:?}", e);
            match e {
                proto::ProtoError::Serial(err) => {
                    let port = matches
                        .get_one::<PathBuf>("device")
                        .expect("Requires device parameter")
                        .display();

                    if err.kind() == tokio_serial::ErrorKind::NoDevice
                        || matches!(err.kind(), tokio_serial::ErrorKind::Io(ErrorKind::NotFound))
                    {
                        eprintln!("{}: File not found", port);
                    } else {
                        eprintln!("I/O Error: {} [device: {}]", err, port,);
                    }
                    exit(-1);
                }
                proto::ProtoError::Io(err) => {
                    let port = matches
                        .get_one::<PathBuf>("device")
                        .expect("Requires device parameter")
                        .display();

                    if err.kind() == ErrorKind::NotFound {
                        eprintln!("{}: File not found", port);
                    } else {
                        eprintln!("I/O Error: {} [device: {}]", err, port,);
                    }
                    exit(-1);
                }
                proto::ProtoError::SyntaxError => {
                    eprintln!("Command was not recognized by device, aborting!");
                    exit(-1);
                }
                proto::ProtoError::ExecutionError => {
                    eprintln!("Command was not executed, maybe device is locked? Try to exit the current screen mode.");
                    exit(-1);
                }
                proto::ProtoError::Abort => {
                    eprintln!("Failed to communicate with device, aborting!");
                    exit(-1);
                }
                proto::ProtoError::Unexpected(_err) => {
                    eprintln!("Received an unexpected response from device, aborting!");
                    exit(-1);
                }
            }
        }
    }

    Ok(())
}

async fn handle_args(matches: &clap::ArgMatches) -> Result<()> {
    let baud_rate = matches
        .get_one::<u32>("baudrate")
        .unwrap_or(&DEFAULT_BAUDRATE);

    if let Some(port_path) = matches.get_one::<PathBuf>("device") {
        let mut device = Device::new(port_path.to_string_lossy().to_string(), *baud_rate)?;

        eprintln!("Connected to: {}\n", port_path.display());

        match matches.subcommand() {
            // Device ID
            Some(("ident", _args)) => {
                let ident = device.ident().await?;
                println!("Model: {}", ident.model);
                println!("Firmware: {}", ident.firmware);
                println!("Serial: {}", ident.serial);
            }
            // Auto Backlight Timeout
            Some(("backlight", args)) => {
                if let Some(minutes) = args.get_one::<String>("minutes") {
                    // Write value
                    let allowed = vec!["5", "10", "15", "20", "25", "30", "off"];
                    if allowed.contains(&minutes.to_lowercase().as_str()) {
                        let duration =
                            Duration::from_secs(minutes.parse::<u64>().unwrap_or(0) * 60);
                        device.set_backlight(duration).await?;
                        println!("OK");
                    } else {
                        eprintln!("Invalid value: {}", minutes);
                    }
                } else {
                    // Read value
                    let backlight = device.backlight().await?;
                    if backlight.is_zero() {
                        println!("Auto Backlight Timeout: OFF");
                    } else {
                        println!("Auto Backlight Timeout: {} min", backlight.as_secs() / 60);
                    }
                }
            }
            // Auto poweroff
            Some(("poweroff", args)) => {
                if let Some(minutes) = args.get_one::<String>("minutes") {
                    // Write value
                    let allowed = vec!["15", "25", "35", "45", "60", "off"];
                    if allowed.contains(&minutes.to_lowercase().as_str()) {
                        let duration =
                            Duration::from_secs(minutes.parse::<u64>().unwrap_or(0) * 60);
                        device.set_poweroff(duration).await?;
                        println!("OK");
                    } else {
                        eprintln!("Invalid value: {}", minutes);
                    }
                } else {
                    // Read value
                    let poweroff = device.poweroff().await?;
                    if poweroff.is_zero() {
                        println!("Auto Power Off: OFF");
                    } else {
                        println!("Auto Power Off: {} min", poweroff.as_secs() / 60);
                    }
                }
            }
            // Operator
            Some(("operator", args)) => {
                if let Some(name) = args.get_one::<String>("name") {
                    // Write value
                    device.set_operator(name).await?;
                    println!("OK");
                } else {
                    // Read value
                    let operator = device.operator().await?;
                    println!("Operator: {}", operator);
                }
            }
            // Copmany
            Some(("company", args)) => {
                if let Some(name) = args.get_one::<String>("name") {
                    // Write value
                    device.set_company(name).await?;
                    println!("OK");
                } else {
                    // Read value
                    let operator = device.company().await?;
                    println!("Operator: {}", operator);
                }
            }
            // Site
            Some(("site", args)) => {
                if let Some(name) = args.get_one::<String>("name") {
                    // Write value
                    device.set_site(name).await?;
                    println!("OK");
                } else {
                    // Read value
                    let operator = device.site().await?;
                    println!("Operator: {}", operator);
                }
            }
            // Contact
            Some(("contact", args)) => {
                if let Some(name) = args.get_one::<String>("name") {
                    // Write value
                    device.set_contact(name).await?;
                    println!("OK");
                } else {
                    // Read value
                    let operator = device.contact().await?;
                    println!("Operator: {}", operator);
                }
            }
            // Clock
            Some(("clock", args)) => {
                if let Some(true) = args.get_one::<bool>("sync-with-host") {
                    // Write value
                    device.set_clock(SystemTime::now()).await?;
                    println!("OK");
                } else {
                    // Read value
                    let clock = device.clock().await?;
                    let system_time = std::time::UNIX_EPOCH + Duration::from_secs(clock);
                    let datetime: DateTime<chrono::Utc> = system_time.into();
                    println!("Device clock: {}", datetime.naive_local());
                }
            }
            _ => {
                todo!()
            }
        }
    }

    Ok(())
}
