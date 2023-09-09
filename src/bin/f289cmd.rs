#![deny(clippy::unwrap_used)]

use chrono::{DateTime, Local};
use clap::builder::BoolishValueParser;
use clap::{arg, command, value_parser};
use f289ctrl::device::ValueMaps;
use f289ctrl::measurement::Reading;
use f289ctrl::proto::command::{
    ClearMemory, DateFormat, DezibelReference, DigitCount, Language, NumericFormat, TimeFormat,
};
use f289ctrl::{proto, DEFAULT_BAUDRATE, DEFAULT_TTY};
use std::io::{ErrorKind, Write};
use std::process::exit;
use std::{env, path::PathBuf, str, time::Duration};

use f289ctrl::device::Device;
use f289ctrl::measurement::{
    Measurement, Memory, Mode, PrimaryFunction, SavedMeasurement, SavedMinMaxMeasurement,
    SavedRecordingSessionInfo, SecondaryFunction, SessionRecordReadings,
};
use f289ctrl::proto::conv::pretty_ts;
use f289ctrl::proto::Result;

#[tokio::main]
async fn main() -> tokio_serial::Result<()> {
    let matches =
        command!() // requires `cargo` feature
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
            .subcommand(clap::Command::new("reset-device").about("Reset device"))
            .subcommand(
                clap::Command::new("custom-dBm")
                    .about("Custom dBm reference in Ohm")
                    .arg(arg!([reference] "Set custom reference").value_parser(value_parser!(u16))),
            )
            .subcommand(
                clap::Command::new("temp-offset")
                    .about("Temperature offset")
                    .arg(arg!([offset] "Set custom offset").value_parser(value_parser!(i16))),
            )
            .subcommand(clap::Command::new("digits").about("Digit count").arg(
                arg!([digits] "Set display digit count").value_parser(value_parser!(DigitCount)),
            ))
            .subcommand(
                clap::Command::new("language")
                    .about("Multimeter language")
                    .arg(arg!([language] "Set language").value_parser(value_parser!(Language))),
            )
            .subcommand(
                clap::Command::new("date-format")
                    .about("Date format")
                    .arg(arg!([fmt] "Set format").value_parser(value_parser!(DateFormat))),
            )
            .subcommand(
                clap::Command::new("time-format")
                    .about("Time format")
                    .arg(arg!([fmt] "Set format").value_parser(value_parser!(TimeFormat))),
            )
            .subcommand(
                clap::Command::new("numeric-format")
                    .about("Numeric format")
                    .arg(arg!([fmt] "Set format").value_parser(value_parser!(NumericFormat))),
            )
            .subcommand(
                clap::Command::new("autohold-event-thd")
                    .about("Autohold event threshold in %")
                    .arg(arg!([percent] "Set threshold").value_parser(value_parser!(u8))),
            )
            .subcommand(
                clap::Command::new("recording-event-thd")
                    .about("Recording event threshold in %")
                    .arg(arg!([percent] "Set threshold").value_parser(value_parser!(u8))),
            )
            .subcommand(
                clap::Command::new("dBm-reference")
                    .about("dBm reference in Ohm")
                    .arg(
                        arg!([reference] "Set dBm reference")
                            .value_parser(value_parser!(DezibelReference)),
                    ),
            )
            .subcommand(
                clap::Command::new("smoothing")
                    .about("Smoothing (AC)")
                    .arg(arg!([state] "Set smoothing").value_parser(BoolishValueParser::new())),
            )
            .subcommand(clap::Command::new("ident").about("Device identification"))
            .subcommand(
                clap::Command::new("beeper")
                    .about("Beeper")
                    .arg(arg!([state] "Set beeper").value_parser(BoolishValueParser::new())),
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
            .subcommand(
                clap::Command::new("mea")
                    //.alias("mea")
                    .about("Get current measurement")
                    .arg(arg!(
                        --"watch" "Poll current measurement forever"
                    )),
            )
            .subcommand(
                clap::Command::new("memory-name")
                    .about("Get/set memory slot name")
                    .arg(arg!(<slot> "Slot").value_parser(clap::value_parser!(u16).range(1..=8)))
                    .arg(arg!([name] "Set name (max 16 chars)")),
            )
            .subcommand(
                clap::Command::new("clear").about("Clear memory").arg(
                    arg!(--"memory" <memory> "Memory type")
                        .value_parser(value_parser!(ClearMemory))
                        .default_missing_value("all")
                        .default_value("all"),
                ),
            )
            .subcommand(
                clap::Command::new("dump-measurements")
                    .about("Dump memory measurements")
                    .alias("dump-mea"),
            )
            .subcommand(clap::Command::new("dump-minmax").about("Dump memory min/max measurements"))
            .subcommand(clap::Command::new("dump-peak").about("Dump memory peak measurement"))
            .subcommand(
                clap::Command::new("dump-recordings")
                    .about("Dump memory recordings")
                    .alias("dump-rec"),
            )
            .subcommand(clap::Command::new("memory").about("List all memory entries"))
            .subcommand(
                clap::Command::new("get-memory")
                    .about("Query memory saving by name")
                    .arg(
                        arg!(
                            [name] "Name of saving"
                        )
                        .required(true),
                    ),
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
                proto::ProtoError::Unexpected(err) => {
                    eprintln!(
                        "Received an unexpected response from device, aborting!: {:?}",
                        err
                    );
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
        let mut device = Device::new(port_path.to_string_lossy(), *baud_rate)?;

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
                    let allowed = ["5", "10", "15", "20", "25", "30", "off"];
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
                    let allowed = ["15", "25", "35", "45", "60", "off"];
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
                    println!("Company: {}", operator);
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
                    println!("Site: {}", operator);
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
                    println!("Contact: {}", operator);
                }
            }
            // Clock
            Some(("clock", args)) => {
                if let Some(true) = args.get_one::<bool>("sync-with-host") {
                    // Write value
                    device.set_clock(Local::now()).await?;
                    println!("OK");
                } else {
                    // Read value
                    let clock = device.clock().await?;
                    let system_time = std::time::UNIX_EPOCH + Duration::from_secs(clock);
                    let datetime: DateTime<chrono::Utc> = system_time.into();
                    println!("Device clock: {}", datetime.naive_local());
                }
            }
            // Reset
            Some(("reset", _)) => {
                device.reset().await?;
                println!("OK");
            }
            // Beeper
            Some(("beeper", args)) => {
                if let Some(state) = args.get_one::<bool>("state") {
                    // Write value
                    device.set_beeper(*state).await?;
                    println!("OK");
                } else {
                    // Read value
                    let state = device.beeper().await?;
                    println!("Beeper: {}", state);
                }
            }
            // Smoothing
            Some(("smoothing", args)) => {
                if let Some(state) = args.get_one::<bool>("state") {
                    // Write value
                    device.set_smoothing(*state).await?;
                    println!("OK");
                } else {
                    // Read value
                    let state = device.smoothing().await?;
                    println!("AC Smoothing: {}", state);
                }
            }
            // Custom dBm
            Some(("custom-dBm", args)) => {
                if let Some(dbm) = args.get_one::<u16>("reference") {
                    // Write value
                    device.set_custom_dbm(*dbm).await?;
                    println!("OK");
                } else {
                    // Read value
                    let dbm = device.custom_dbm().await?;
                    println!("Custom dBm: {}", dbm);
                }
            }
            // dBm-Ref
            Some(("dBm-reference", args)) => {
                if let Some(dbm) = args.get_one::<DezibelReference>("reference") {
                    // Write value
                    device.set_dbm_ref(*dbm).await?;
                    println!("OK");
                } else {
                    // Read value
                    let dbm = device.dbm_ref().await?;
                    println!("dBm reference: {}", dbm);
                }
            }
            // Temp Offset
            Some(("temp-offset", args)) => {
                if let Some(offset) = args.get_one::<i16>("offset") {
                    // Write value
                    device.set_temp_offset(*offset).await?;
                    println!("OK");
                } else {
                    // Read value
                    let offset = device.temp_offset().await?;
                    println!("Temp. offset: {}", offset);
                }
            }
            // Digit count
            Some(("digits", args)) => {
                if let Some(count) = args.get_one::<DigitCount>("digits") {
                    // Write value
                    device.set_digit_count(*count).await?;
                    println!("OK");
                } else {
                    // Read value
                    let count = device.digit_count().await?;
                    match count {
                        DigitCount::Digit4 => println!("Digit count: 4",),
                        DigitCount::Digit5 => println!("Digit count: 5",),
                    }
                }
            }
            // Numeric format
            Some(("numeric-format", args)) => {
                if let Some(fmt) = args.get_one::<NumericFormat>("fmt") {
                    // Write value
                    device.set_numeric_format(*fmt).await?;
                    println!("OK");
                } else {
                    // Read value
                    let fmt = device.numeric_format().await?;
                    match fmt {
                        NumericFormat::Comma => println!("Numeric format: COMMA",),
                        NumericFormat::Point => println!("Numeric format: POINT",),
                    }
                }
            }
            // Date Format
            Some(("date-format", args)) => {
                if let Some(fmt) = args.get_one::<DateFormat>("fmt") {
                    // Write value
                    device.set_date_format(*fmt).await?;
                    println!("OK");
                } else {
                    // Read value
                    let fmt = device.date_format().await?;
                    match fmt {
                        DateFormat::MM_DD => println!("Date format: MM/DD",),
                        DateFormat::DD_MM => println!("Date format: DD/MM",),
                    }
                }
            }
            // Time Format
            Some(("time-format", args)) => {
                if let Some(fmt) = args.get_one::<TimeFormat>("fmt") {
                    // Write value
                    device.set_time_format(*fmt).await?;
                    println!("OK");
                } else {
                    // Read value
                    let fmt = device.time_format().await?;
                    match fmt {
                        TimeFormat::Time12 => println!("Time format: 12h",),
                        TimeFormat::Time24 => println!("Time format: 24h",),
                    }
                }
            }
            // Language
            Some(("language", args)) => {
                if let Some(lang) = args.get_one::<Language>("language") {
                    // Write value
                    device.set_language(*lang).await?;
                    println!("OK");
                } else {
                    // Read value
                    let lang = match device.language().await? {
                        Language::English => "ENGLISH",
                        Language::German => "GERMAN",
                        Language::French => "FRENCH",
                        Language::Italian => "ITALIAN",
                        Language::Spanish => "SPANISH",
                        Language::Japanese => "JAPANESE",
                        Language::Chinese => "CHINESE",
                    };
                    println!("Language: {}", lang);
                }
            }
            // Autohold event thd
            Some(("autohold-event-thd", args)) => {
                if let Some(thd) = args.get_one::<u8>("percent") {
                    // Write value
                    device.set_autohold_event_threshold(*thd).await?;
                    println!("OK");
                } else {
                    // Read value
                    let thd = device.autohold_event_threshold().await?;
                    println!("Autohold event threshold: {}", thd);
                }
            }
            // Recording event thd
            Some(("recording-event-thd", args)) => {
                if let Some(thd) = args.get_one::<u8>("percent") {
                    // Write value
                    device.set_recording_event_threshold(*thd).await?;
                    println!("OK");
                } else {
                    // Read value
                    let thd = device.recording_event_threshold().await?;
                    println!("Recording event threshold: {}", thd);
                }
            }
            // Clear
            Some(("clear", args)) => {
                if let Some(memory) = args.get_one::<ClearMemory>("memory") {
                    device.clear(*memory).await?;
                    println!("OK");
                } else {
                    panic!("memory arg missing")
                }
            }
            // Measurement
            Some(("mea", args)) => {
                let watch = args.get_one::<bool>("watch").unwrap_or(&false);

                let maps = device.value_maps().await?;

                let mut c = 1;

                let mut prifunction = None;
                let mut secfunction = None;
                let mut modes = None;

                loop {
                    match device.live_measurement().await {
                        Ok(Some(mea_raw)) => {
                            let mea = Measurement::from((mea_raw, &maps));

                            if prifunction != Some(mea.pri_function)
                                || secfunction != Some(mea.sec_function)
                                || modes.as_ref() != Some(&mea.modes)
                            {
                                prifunction = Some(mea.pri_function);
                                secfunction = Some(mea.sec_function);
                                modes = Some(mea.modes.clone());
                                println!(
                                    "Measurement primary: [{}], secondary: [{}], modes: [{}]",
                                    mea.pri_function, mea.sec_function, mea.modes
                                );
                            }
                            for r in &mea.readings {
                                println!(
                                    "#{:0>4}/{:0>4} {:>15} {:>20}",
                                    c,
                                    r.reading_id,
                                    r.to_string(),
                                    r.ts.format("%Y-%m-%d %H:%M:%S")
                                );
                                //println!("{:?}", r);
                            }
                        }
                        Ok(None) => {
                            println!("--- NO DATA ---");
                        }
                        Err(err) => {
                            eprintln!("Error: {}", err);
                        }
                    }

                    if !watch {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    c += 1;
                }
            }
            // memory-name
            Some(("memory-name", args)) => {
                if let Some(name) = args.get_one::<String>("name") {
                    let slot = args.get_one::<u16>("slot").expect("Slot expected");
                    device.set_save_name(slot - 1, name).await?;
                    println!("OK");
                } else {
                    let slot = args.get_one::<u16>("slot").expect("Slot expected");
                    let name = device.save_name(slot - 1).await?;
                    println!("Name[{}]: {}", slot, name);
                }
            }

            Some(("dump-measurements", _args)) => {
                //let watch = args.get_one::<bool>("watch").unwrap_or(&false);

                let maps = device.value_maps().await?;

                let raw_meas = device.saved_measurements_all().await?;

                let meas: Vec<SavedMeasurement> = raw_meas
                    .into_iter()
                    .map(|rm| SavedMeasurement::from((rm, &maps)))
                    .collect();

                for mea in &meas {
                    println!(
                        "Saved Measurement: '{}', primary: {}, secondary: {}",
                        mea.name, mea.pri_function, mea.sec_function,
                    );
                    for reading in &mea.readings {
                        let ext = reading
                            .attribute
                            .as_ref()
                            .map(|attr| format!(" [{}]", attr))
                            .unwrap_or_default();
                        println!("#{:04} {}{:>20}", reading.reading_id, reading, ext);
                    }
                }
            }

            Some(("dump-minmax", _args)) => {
                //let watch = args.get_one::<bool>("watch").unwrap_or(&false);

                let maps = device.value_maps().await?;

                let raw_meas = device.saved_minmax_all().await?;

                let meas: Vec<SavedMinMaxMeasurement> = raw_meas
                    .into_iter()
                    .map(|rm| SavedMinMaxMeasurement::from((rm, &maps)))
                    .collect();

                for mea in &meas {
                    println!(
                        "Saved Min/max Measurement: '{}', primary: {}, secondary: {}",
                        mea.name, mea.pri_function, mea.sec_function,
                    );
                    if mea.readings.len() == 4 {
                        println!(
                            "Min/Max #{}: NOW: {}, MIN: {}, MAX: {}, AVG: {}",
                            mea.seq_no,
                            mea.readings[0],
                            mea.readings[1],
                            mea.readings[2],
                            mea.readings[3]
                        );
                    } else {
                        eprintln!(
                            "Invalid readings count for min/max: {}, expected 4",
                            mea.readings.len()
                        );
                    }
                }
            }

            Some(("dump-peak", _args)) => {
                //let watch = args.get_one::<bool>("watch").unwrap_or(&false);

                let maps = device.value_maps().await?;

                let raw_meas = device.saved_peak_all().await?;

                let meas: Vec<SavedMinMaxMeasurement> = raw_meas
                    .into_iter()
                    .map(|rm| SavedMinMaxMeasurement::from((rm, &maps)))
                    .collect();

                for mea in &meas {
                    println!(
                        "Saved Peak Measurement: '{}', primary: {}, secondary: {}",
                        mea.name, mea.pri_function, mea.sec_function,
                    );
                    if mea.readings.len() == 4 {
                        println!(
                            "Peak #{}: NOW: {}, MIN: {}, MAX: {}, AVG: {}",
                            mea.seq_no,
                            mea.readings[0],
                            mea.readings[1],
                            mea.readings[2],
                            mea.readings[3]
                        );
                    } else {
                        eprintln!(
                            "Invalid readings count for min/max: {}, expected 4",
                            mea.readings.len()
                        );
                    }
                }
            }

            Some(("dump-recordings", _args)) => {
                //let watch = args.get_one::<bool>("watch").unwrap_or(&false);

                let maps = device.value_maps().await?;

                let raw_meas = device.saved_recordings_all().await?;

                let meas: Vec<SavedRecordingSessionInfo> = raw_meas
                    .into_iter()
                    .map(|rm| SavedRecordingSessionInfo::from((rm, &maps)))
                    .collect();

                for mea in &meas {
                    println!(
                        "Saved Recording: '{}', primary: {}, secondary: {}, Samples: {}",
                        mea.name, mea.pri_function, mea.sec_function, mea.num_samples,
                    );

                    //for reading in &mea.readings {
                    //    println!("#{:0>4} {}", mea.seq_no, reading.value);
                    //}
                    let rr = device
                        .session_record_reading_all_cb(
                            mea.reading_index as usize,
                            mea.num_samples as usize,
                            |index, total| {
                                print!("\rReading {}/{}", index, total);
                                std::io::stdout().flush().expect("Unable to flush stdout");
                            },
                        )
                        .await?;
                    print!("\r");

                    let recordings: Vec<SessionRecordReadings> = rr
                        .into_iter()
                        .map(|rm| SessionRecordReadings::try_from((rm, &maps)))
                        .collect::<std::result::Result<Vec<_>, _>>()?;

                    for rec in &recordings {
                        let mut avg = rec.span_readings[2].clone();
                        avg.value /= rec.sampling as f64;

                        let duration = {
                            let diff = (rec.end_ts - rec.start_ts)
                                .to_std()
                                .expect("Invalid timestamp from device");
                            let seconds = ((diff.as_millis() as f64) % (1000.0 * 60.0)) / 1000.0;
                            let minutes = (diff.as_secs() / 60) % 60;
                            let hours = (diff.as_secs() / 60) / 60;
                            format!("{:02}:{:02}:{:02.1}", hours, minutes, seconds).to_string()
                        };

                        println!(
                            "[{ts_start}]{value:#8} {duration:>10}, min({min_ts}): {min:8}, avg: {avg:8}, max({max_ts}): {max:8} [{record_type}{stable}]",
                            value = rec.fixed_reading,
                            ts_start = pretty_ts(&rec.start_ts),
                            duration = duration,
                            min = rec.span_readings[1],
                            min_ts = pretty_ts(&rec.span_readings[1].ts),
                            avg = avg,
                            max = rec.span_readings[0],
                            max_ts = pretty_ts(&rec.span_readings[0].ts),
                            //ts_end = pretty_ts(&rec.end_ts),
                            record_type = rec.record_type,
                            stable = if rec.stable.0 { ",Stable" } else {""},
                        );
                    }

                    /*
                    for readings in &rr {
                        //println!("New RecReading: {:?}", readings);
                        for reading in &readings.readings {
                            let r = Reading::from((reading.clone(), &maps));
                            println!("#{:0>4} {} {:?}", r.reading_id, r.ts, r);
                        }
                        println!(
                            "##{:0>4} {}, {}",
                            readings.reading2.reading_id,
                            timestamp_to_datetime(readings.reading2.ts),
                            readings.reading2.value
                        );
                    }
                     */
                    println!();
                }
            }
            Some(("memory", _args)) => {
                //let watch = args.get_one::<bool>("watch").unwrap_or(&false);

                let maps = device.value_maps().await?;

                let stats = device.memory_statistics().await?;
                let memory = device.all_memory(&maps).await?;

                println!("Saved measurements: {}", stats.measurement);
                memory.iter().for_each(|entry| {
                    if let Memory::Measurement(mea) = entry {
                        println!(
                            "{} {:<30} {}",
                            pretty_ts(&mea.readings[0].ts),
                            quoted_string(&mea.name),
                            mea.pri_function
                        );
                    }
                });
                println!();

                println!("Saved min/max measurements: {}", stats.min_max);
                memory.iter().for_each(|entry| {
                    if let Memory::MinMaxMeasurement(mea) = entry {
                        println!(
                            "{} {:<30} {}",
                            pretty_ts(&mea.ts1),
                            quoted_string(&mea.name),
                            mea.pri_function
                        );
                    }
                });
                println!();

                println!("Saved peak measurements: {}", stats.peak);
                memory.iter().for_each(|entry| {
                    if let Memory::PeakMeasurement(mea) = entry {
                        println!(
                            "{} {:<30} {}",
                            pretty_ts(&mea.ts1),
                            quoted_string(&mea.name),
                            mea.readings[0]
                        );
                    }
                });
                println!();

                println!("Saved recordings: {}", stats.recordings);
                memory.iter().for_each(|entry| {
                    if let Memory::Recording(mea) = entry {
                        println!(
                            "{} {:<30} {}",
                            pretty_ts(&mea.start_ts),
                            quoted_string(&mea.name),
                            mea.pri_function
                        );
                    }
                });
            }
            Some(("get-memory", args)) => {
                //let watch = args.get_one::<bool>("watch").unwrap_or(&false);

                let name = args.get_one::<String>("name").expect("name parameter");

                let maps = device.value_maps().await?;

                match device
                    .all_memory(&maps)
                    .await?
                    .iter()
                    .find(|entry| entry.name() == name)
                {
                    Some(Memory::Measurement(m)) => {
                        pretty_measurement(&mut device, m).await?;
                    }
                    Some(Memory::MinMaxMeasurement(m)) => {
                        pretty_minmax_or_peak_measurement(&mut device, m, false).await?;
                    }
                    Some(Memory::PeakMeasurement(m)) => {
                        pretty_minmax_or_peak_measurement(&mut device, m, true).await?;
                    }
                    Some(Memory::Recording(m)) => {
                        pretty_recording(&mut device, m, &maps).await?;
                    }
                    None => {
                        println!("'{}' not found", name);
                    }
                }
            }
            _ => {
                todo!()
            }
        }
    }

    Ok(())
}

fn quoted_string(s: impl AsRef<str>) -> String {
    String::from("\"") + s.as_ref() + "\""
}

async fn pretty_measurement(_device: &mut Device, mea: &SavedMeasurement) -> Result<()> {
    println!(
        "Saved Measurement: '{}', primary: {}, secondary: {}, modes: [{}]",
        mea.name, mea.pri_function, mea.sec_function, mea.modes
    );
    let mut processed = 0;
    pretty_value("Pri.", &mea.readings[0]);
    processed += 1;
    if mea.sec_function != SecondaryFunction::None {
        pretty_value("Sec.", &mea.readings[1]);
        processed += 1;
    }

    if mea.modes.is(Mode::Rel) || mea.modes.is(Mode::RelPercent) {
        pretty_value("Value", &mea.readings[1]);
        pretty_value("Reference", &mea.readings[2]);
    } else if mea.readings.len() > processed {
        println!("Additional readings:");
        for (i, reading) in mea.readings.iter().skip(processed).enumerate() {
            pretty_value(format!("#{:>03}", i + processed), reading);
        }
    }
    Ok(())
}

async fn pretty_minmax_or_peak_measurement(
    _device: &mut Device,
    mea: &SavedMinMaxMeasurement,
    peak: bool,
) -> Result<()> {
    let bolt = if mea.bolt.0 { " 🗲 " } else { "" };

    if peak {
        println!(
            "Saved Peak Measurement: '{}',{} primary: {}, secondary: {}, modes: [{}]",
            mea.name, bolt, mea.pri_function, mea.sec_function, mea.modes
        );
    } else {
        println!(
            "Saved Min/Max Measurement: '{}',{} primary: {}, secondary: {}, modes: [{}]",
            mea.name, bolt, mea.pri_function, mea.sec_function, mea.modes
        );
    }

    //println!("{:?}", mea);

    println!("Started at: {}", pretty_ts(&mea.ts1));
    if mea.pri_function == PrimaryFunction::CAPACITANCE
        && (mea.modes.is(Mode::RelPercent) || mea.modes.is(Mode::Rel))
    {
        let value = &mea.readings[0];
        let reference1 = &mea.readings[1];
        let reference2 = &mea.readings[5];
        let min = &mea.readings[2];
        let max = &mea.readings[3];
        let avg = &mea.readings[4];
        pretty_value("Value", value);
        pretty_value("Ref1", reference1);
        pretty_value("Ref2", reference2);
        pretty_value("Min", min);
        pretty_value("Max", max);
        pretty_value("Avg", avg);
    } else if mea.sec_function == SecondaryFunction::DbmHertz
        || mea.sec_function == SecondaryFunction::DbvHertz
    {
        let value = &mea.readings[0];
        let reference = &mea.readings[5];
        let hertz = &mea.readings[1];
        let min = &mea.readings[2];
        let max = &mea.readings[3];
        let avg = &mea.readings[4];
        pretty_value("Value", value);
        pretty_value("Ref", reference);
        pretty_value("Hertz", hertz);
        pretty_value("Min", min);
        pretty_value("Max", max);
        pretty_value("Avg", avg);
    } else if mea.sec_function == SecondaryFunction::Dbm
        || mea.sec_function == SecondaryFunction::Dbv
    {
        let value = &mea.readings[0];
        let reference = &mea.readings[5];
        let vac = &mea.readings[1];
        let min = &mea.readings[2];
        let max = &mea.readings[3];
        let avg = &mea.readings[4];
        pretty_value("Value", value);
        pretty_value("Ref", reference);
        pretty_value("VAC", vac);
        pretty_value("Min", min);
        pretty_value("Max", max);
        pretty_value("Avg", avg);
    } else if mea.sec_function == SecondaryFunction::CrestFactor {
        let value = &mea.readings[0];
        let reference = &mea.readings[1];
        let min = &mea.readings[2];
        let max = &mea.readings[3];
        let avg = &mea.readings[4];
        pretty_value("Value", value);
        pretty_value("Ref", reference);
        pretty_value("Min", min);
        pretty_value("Max", max);
        pretty_value("Avg", avg);
    } else if mea.sec_function == SecondaryFunction::PulseWidth
        || mea.sec_function == SecondaryFunction::DutyCycle
        || mea.sec_function == SecondaryFunction::Hertz
    {
        let value = &mea.readings[0];
        let hertz = &mea.readings[1];
        let min = &mea.readings[2];
        let max = &mea.readings[3];
        let avg = &mea.readings[4];
        pretty_value("Value", value);
        pretty_value("Hertz", hertz);
        pretty_value("Min", min);
        pretty_value("Max", max);
        pretty_value("Avg", avg);
    } else if mea.modes.is(Mode::RelPercent)
        || mea.modes.is(Mode::Rel)
        || mea.sec_function == SecondaryFunction::DbmHertz
    {
        let value = &mea.readings[0];
        let reference = &mea.readings[1];
        let min = &mea.readings[2];
        let max = &mea.readings[3];
        let avg = &mea.readings[4];
        pretty_value("Value", value);
        pretty_value("Ref", reference);
        pretty_value("Min", min);
        pretty_value("Max", max);
        pretty_value("Avg", avg);
    } else if mea.pri_function == PrimaryFunction::A_AC_PLUS_DC
        || mea.pri_function == PrimaryFunction::MA_AC_PLUS_DC
        || mea.pri_function == PrimaryFunction::UA_AC_PLUS_DC
        || mea.pri_function == PrimaryFunction::V_AC_PLUS_DC
        || mea.pri_function == PrimaryFunction::MV_AC_PLUS_DC
    {
        let value = &mea.readings[0];
        let _ = &mea.readings[1];
        let min = &mea.readings[2];
        let max = &mea.readings[3];
        let avg = &mea.readings[4];
        pretty_value("Value", value);
        //pretty_value("Ref", reference);
        pretty_value("Min", min);
        pretty_value("Max", max);
        pretty_value("Avg", avg);
    } else {
        let value = &mea.readings[0];
        let min = &mea.readings[1];
        let max = &mea.readings[2];
        let avg = &mea.readings[3];
        pretty_value("Value", value);
        pretty_value("Min", min);
        pretty_value("Max", max);
        pretty_value("Avg", avg);
    }
    println!("Stopped at: {}", pretty_ts(&mea.ts2));
    Ok(())
}

async fn pretty_recording(
    device: &mut Device,
    mea: &SavedRecordingSessionInfo,
    maps: &ValueMaps,
) -> Result<()> {
    println!(
        "Saved Recording: '{}', primary: {}, secondary: {}, Samples: {}",
        mea.name, mea.pri_function, mea.sec_function, mea.num_samples,
    );

    //for reading in &mea.readings {
    //    println!("#{:0>4} {}", mea.seq_no, reading.value);
    //}
    let rr = device
        .session_record_reading_all_cb(
            mea.reading_index as usize,
            mea.num_samples as usize,
            |index, total| {
                print!("\rReading {}/{}", index, total);
                std::io::stdout().flush().expect("Unable to flush stdout");
            },
        )
        .await?;
    println!();

    let recordings: Vec<SessionRecordReadings> = rr
        .into_iter()
        .map(|rm| SessionRecordReadings::try_from((rm, maps)))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    for rec in &recordings {
        let mut avg = rec.span_readings[2].clone();
        avg.value /= rec.sampling as f64;

        //println!("{:?}", rec);

        let duration = {
            let diff = (rec.end_ts - rec.start_ts)
                .to_std()
                .expect("Invalid timestamp from device");
            let seconds = ((diff.as_millis() as f64) % (1000.0 * 60.0)) / 1000.0;
            let minutes = (diff.as_secs() / 60) % 60;
            let hours = (diff.as_secs() / 60) / 60;
            format!("{:02}:{:02}:{:02.1}", hours, minutes, seconds).to_string()
        };

        println!(
            "[{ts_start}]{value:#8} {duration:>10}, min({min_ts}): {min:8}, avg: {avg:8}, max({max_ts}): {max:8} [{record_type}{stable}]",
            value = rec.fixed_reading,
            ts_start = pretty_ts(&rec.start_ts),
            duration = duration,
            min = rec.span_readings[1],
            min_ts = pretty_ts(&rec.span_readings[1].ts),
            avg = avg,
            max = rec.span_readings[0],
            max_ts = pretty_ts(&rec.span_readings[0].ts),
            //ts_end = pretty_ts(&rec.end_ts),
            record_type = rec.record_type,
            stable = if rec.stable.0 { ",Stable" } else {""},
        );
    }
    Ok(())
}

fn pretty_value(caption: impl AsRef<str>, reading: &Reading) {
    let block1 = format!("{:10} {:#8}", caption.as_ref().to_string() + ":", reading);
    println!("{:<35} [{}]", block1, pretty_ts(&reading.ts));
}
