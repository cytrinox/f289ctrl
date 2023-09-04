use std::fmt;

use chrono::{DateTime, Utc};

use crate::{
    device::ValueMaps,
    proto::conv::{timestamp_to_datetime, unit_prefix},
    rawmea::{
        RawMeasurement, RawReading, RawSavedMeasurement, RawSavedMinMaxMeasurement,
        RawSavedRecordingSessionInfo, RawSessionRecordReadings,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum PrimaryFunction {
    V_DC,
    TEMPERATURE,
    A_DC,
    V_DC_OVER_AC,
    V_AC_OVER_DC,
    CAL_ACDC_AC_COMP,
    CAL_V_AC_LOZ,
    LIMBO,
    V_AC_LOZ,
    OHMS_LOW,
    CAL_RMS,
    CAL_TEMPERATURE,
    CAPACITANCE,
    OHMS,
    MA_AC,
    V_AC_PLUS_DC,
    MV_AC_PLUS_DC,
    MA_DC_OVER_AC,
    CAL_AD_GAIN_X2,
    CAL_DC_AMP_X5,
    MV_DC_OVER_AC,
    A_AC,
    CONTINUITY,
    MV_AC,
    MV_DC,
    A_DC_OVER_AC,
    CONDUCTANCE,
    V_AC,
    CAL_AD_GAIN_X1,
    CAL_DC_AMP_X10,
    UA_AC_PLUS_DC,
    UA_DC_OVER_AC,
    CAL_NINV_AC_AMP,
    CAL_ISRC_500NA,
    UA_DC,
    UA_AC_OVER_DC,
    A_AC_OVER_DC,
    CAL_FILT_AMP,
    MA_AC_OVER_DC,
    MA_AC_PLUS_DC,
    CAL_MV_AC_PEAK,
    UA_AC,
    MV_AC_OVER_DC,
    CAL_V_DC_LOZ,
    MA_DC,
    DIODE_TEST,
    CAL_COMP_TRIM_MV_DC,
    CAL_V_AC_PEAK,
    A_AC_PLUS_DC,
}

impl fmt::Display for PrimaryFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimaryFunction::V_DC => f.write_str("V DC"),
            PrimaryFunction::TEMPERATURE => f.write_str("Temperature"),
            PrimaryFunction::A_DC => f.write_str("A DC"),
            PrimaryFunction::V_DC_OVER_AC => f.write_str("V DC,AC"),
            PrimaryFunction::V_AC_OVER_DC => f.write_str("V AC,DC"),
            PrimaryFunction::CAL_ACDC_AC_COMP => f.write_str("Calibrate AC/DC AC COMP"),
            PrimaryFunction::CAL_V_AC_LOZ => f.write_str("Calibrate V AC LOZ"),
            PrimaryFunction::LIMBO => f.write_str("LIMBO"),
            PrimaryFunction::V_AC_LOZ => f.write_str("V AC LoZ"),
            PrimaryFunction::OHMS_LOW => f.write_str("LoΩ"),
            PrimaryFunction::CAL_RMS => f.write_str("Calibrate RMS"),
            PrimaryFunction::CAL_TEMPERATURE => f.write_str("Calibrate temperature"),
            PrimaryFunction::CAPACITANCE => f.write_str("Capacity"),
            PrimaryFunction::OHMS => f.write_str("Ohms"),
            PrimaryFunction::MA_AC => f.write_str("mA AC"),
            PrimaryFunction::V_AC_PLUS_DC => f.write_str("V AC+DC"),
            PrimaryFunction::MV_AC_PLUS_DC => f.write_str("mV AC+DC"),
            PrimaryFunction::MA_DC_OVER_AC => f.write_str("mA DC,AC"),
            PrimaryFunction::CAL_AD_GAIN_X2 => f.write_str("Calibrate AD"),
            PrimaryFunction::CAL_DC_AMP_X5 => f.write_str("Calibrate DC"),
            PrimaryFunction::MV_DC_OVER_AC => f.write_str("mV DC,AC"),
            PrimaryFunction::A_AC => f.write_str("A AC"),
            PrimaryFunction::CONTINUITY => f.write_str("Continuity"),
            PrimaryFunction::MV_AC => f.write_str("mV AC"),
            PrimaryFunction::MV_DC => f.write_str("mv DC"),
            PrimaryFunction::A_DC_OVER_AC => f.write_str("A DC,AC"),
            PrimaryFunction::CONDUCTANCE => f.write_str("Conductivity"),
            PrimaryFunction::V_AC => f.write_str("V AC"),
            PrimaryFunction::CAL_AD_GAIN_X1 => f.write_str("Calibrate AD"),
            PrimaryFunction::CAL_DC_AMP_X10 => f.write_str("Calibrate DC"),
            PrimaryFunction::UA_AC_PLUS_DC => f.write_str("µA AC+DC"),
            PrimaryFunction::UA_DC_OVER_AC => f.write_str("µA DC,AC"),
            PrimaryFunction::CAL_NINV_AC_AMP => f.write_str("Calibrate NINV"),
            PrimaryFunction::CAL_ISRC_500NA => f.write_str("Calibrate ISRC"),
            PrimaryFunction::UA_DC => f.write_str("µA DC"),
            PrimaryFunction::UA_AC_OVER_DC => f.write_str("µA AC,DC"),
            PrimaryFunction::A_AC_OVER_DC => f.write_str("A AC,DC"),
            PrimaryFunction::CAL_FILT_AMP => f.write_str("Calibrate Filter"),
            PrimaryFunction::MA_AC_OVER_DC => f.write_str("mA AC,DC"),
            PrimaryFunction::MA_AC_PLUS_DC => f.write_str("mA AC+DC"),
            PrimaryFunction::CAL_MV_AC_PEAK => f.write_str("Calibrate MV"),
            PrimaryFunction::UA_AC => f.write_str("µA AC"),
            PrimaryFunction::MV_AC_OVER_DC => f.write_str("mV AC,DC"),
            PrimaryFunction::CAL_V_DC_LOZ => f.write_str("Calibrate V DC LoZ"),
            PrimaryFunction::MA_DC => f.write_str("mA DC"),
            PrimaryFunction::DIODE_TEST => f.write_str("Diode test"),
            PrimaryFunction::CAL_COMP_TRIM_MV_DC => f.write_str("Calibrate COMP"),
            PrimaryFunction::CAL_V_AC_PEAK => f.write_str("Calibrate V AC Peak"),
            PrimaryFunction::A_AC_PLUS_DC => f.write_str("A AC+DC"),
        }
    }
}

impl From<(u16, &ValueMaps)> for PrimaryFunction {
    // "primfunction": {3: "V_DC", 26: "TEMPERATURE", 14: "A_DC", 6: "V_DC_OVER_AC",
    // 5: "V_AC_OVER_DC", 44: "CAL_ACDC_AC_COMP", 45: "CAL_V_AC_LOZ", 0: "LIMBO",
    // 32: "V_AC_LOZ", 33: "OHMS_LOW", 37: "CAL_RMS", 48: "CAL_TEMPERATURE",
    // 30: "CAPACITANCE", 27: "OHMS", 12: "MA_AC", 7: "V_AC_PLUS_DC", 10: "MV_AC_PLUS_DC",
    // 21: "MA_DC_OVER_AC", 35: "CAL_AD_GAIN_X2", 39: "CAL_DC_AMP_X5", 9: "MV_DC_OVER_AC",
    // 11: "A_AC", 29: "CONTINUITY", 2: "MV_AC", 4: "MV_DC", 18: "A_DC_OVER_AC",
    // 28: "CONDUCTANCE", 1: "V_AC", 36: "CAL_AD_GAIN_X1", 40: "CAL_DC_AMP_X10",
    // 25: "UA_AC_PLUS_DC", 24: "UA_DC_OVER_AC", 41: "CAL_NINV_AC_AMP", 42: "CAL_ISRC_500NA",
    // 16: "UA_DC", 23: "UA_AC_OVER_DC", 17: "A_AC_OVER_DC", 38: "CAL_FILT_AMP",
    // 20: "MA_AC_OVER_DC", 22: "MA_AC_PLUS_DC", 47: "CAL_MV_AC_PEAK", 13: "UA_AC",
    // 8: "MV_AC_OVER_DC", 34: "CAL_V_DC_LOZ", 15: "MA_DC", 31: "DIODE_TEST",
    // 43: "CAL_COMP_TRIM_MV_DC", 46: "CAL_V_AC_PEAK", 19: "A_AC_PLUS_DC"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["primfunction"].get(&value.0).map(String::as_str) {
            Some("V_DC") => Self::V_DC,
            Some("TEMPERATURE") => Self::TEMPERATURE,
            Some("A_DC") => Self::A_DC,
            Some("V_DC_OVER_AC") => Self::V_DC_OVER_AC,
            Some("V_AC_OVER_DC") => Self::V_AC_OVER_DC,
            Some("CAL_ACDC_AC_COMP") => Self::CAL_ACDC_AC_COMP,
            Some("CAL_V_AC_LOZ") => Self::CAL_V_AC_LOZ,
            Some("LIMBO") => Self::LIMBO,
            Some("V_AC_LOZ") => Self::V_AC_LOZ,
            Some("OHMS_LOW") => Self::OHMS_LOW,
            Some("CAL_RMS") => Self::CAL_RMS,
            Some("CAL_TEMPERATURE") => Self::CAL_TEMPERATURE,
            Some("CAPACITANCE") => Self::CAPACITANCE,
            Some("OHMS") => Self::OHMS,
            Some("MA_AC") => Self::MA_AC,
            Some("V_AC_PLUS_DC") => Self::V_AC_PLUS_DC,
            Some("MV_AC_PLUS_DC") => Self::MV_AC_PLUS_DC,
            Some("MA_DC_OVER_AC") => Self::MA_DC_OVER_AC,
            Some("CAL_AD_GAIN_X2") => Self::CAL_AD_GAIN_X2,
            Some("CAL_DC_AMP_X5") => Self::CAL_DC_AMP_X5,
            Some("MV_DC_OVER_AC") => Self::MV_DC_OVER_AC,
            Some("A_AC") => Self::A_AC,
            Some("CONTINUITY") => Self::CONTINUITY,
            Some("MV_AC") => Self::MV_AC,
            Some("MV_DC") => Self::MV_DC,
            Some("A_DC_OVER_AC") => Self::A_DC_OVER_AC,
            Some("CONDUCTANCE") => Self::CONDUCTANCE,
            Some("V_AC") => Self::V_AC,
            Some("CAL_AD_GAIN_X1") => Self::CAL_AD_GAIN_X1,
            Some("CAL_DC_AMP_X10") => Self::CAL_DC_AMP_X10,
            Some("UA_AC_PLUS_DC") => Self::UA_AC_PLUS_DC,
            Some("UA_DC_OVER_AC") => Self::UA_DC_OVER_AC,
            Some("CAL_NINV_AC_AMP") => Self::CAL_NINV_AC_AMP,
            Some("CAL_ISRC_500NA") => Self::CAL_ISRC_500NA,
            Some("UA_DC") => Self::UA_DC,
            Some("UA_AC_OVER_DC") => Self::UA_AC_OVER_DC,
            Some("A_AC_OVER_DC") => Self::A_AC_OVER_DC,
            Some("CAL_FILT_AMP") => Self::CAL_FILT_AMP,
            Some("MA_AC_OVER_DC") => Self::MA_AC_OVER_DC,
            Some("MA_AC_PLUS_DC") => Self::MA_AC_PLUS_DC,
            Some("CAL_MV_AC_PEAK") => Self::CAL_MV_AC_PEAK,
            Some("UA_AC") => Self::UA_AC,
            Some("MV_AC_OVER_DC") => Self::MV_AC_OVER_DC,
            Some("CAL_V_DC_LOZ") => Self::CAL_V_DC_LOZ,
            Some("MA_DC") => Self::MA_DC,
            Some("DIODE_TEST") => Self::DIODE_TEST,
            Some("CAL_COMP_TRIM_MV_DC") => Self::CAL_COMP_TRIM_MV_DC,
            Some("CAL_V_AC_PEAK") => Self::CAL_V_AC_PEAK,
            Some("A_AC_PLUS_DC") => Self::A_AC_PLUS_DC,
            Some(x) => panic!("Unknown primfunction: {}", x),
            None => panic!("Unknown primfunction index: {}", value.0),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum SecondaryFunction {
    DbmHertz,
    None,
    Dbm,
    Hertz,
    DbvHertz,
    DutyCycle,
    CrestFactor,
    PeakMinMax,
    Dbv,
    PulseWidth,
}

impl fmt::Display for SecondaryFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecondaryFunction::DbmHertz => f.write_str("dBm Hertz"),
            SecondaryFunction::None => f.write_str("None"),
            SecondaryFunction::Dbm => f.write_str("dBm"),
            SecondaryFunction::Hertz => f.write_str("Hertz"),
            SecondaryFunction::DbvHertz => f.write_str("dBV Hertz"),
            SecondaryFunction::DutyCycle => f.write_str("Duty Cycle"),
            SecondaryFunction::CrestFactor => f.write_str("Crest Factor"),
            SecondaryFunction::PeakMinMax => f.write_str("Peak Min/Max"),
            SecondaryFunction::Dbv => f.write_str("dBV"),
            SecondaryFunction::PulseWidth => f.write_str("Pulse width"),
        }
    }
}

impl From<(u16, &ValueMaps)> for SecondaryFunction {
    // "secfunction": {6: "DBM_HERTZ", 0: "NONE", 4: "DBM", 1: "HERTZ"
    // 7: "DBV_HERTZ", 2: "DUTY_CYCLE", 8: "CREST_FACTOR",
    // 9: "PEAK_MIN_MAX", 5: "DBV", 3: "PULSE_WIDTH"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["secfunction"].get(&value.0).map(String::as_str) {
            Some("DBM_HERTZ") => Self::DbmHertz,
            Some("NONE") => Self::None,
            Some("DBM") => Self::Dbm,
            Some("HERTZ") => Self::Hertz,
            Some("DBV_HERTZ") => Self::DbvHertz,
            Some("DUTY_CYCLE") => Self::DutyCycle,
            Some("CREST_FACTOR") => Self::CrestFactor,
            Some("PEAK_MIN_MAX") => Self::PeakMinMax,
            Some("DBV") => Self::Dbv,
            Some("PULSE_WIDTH") => Self::PulseWidth,
            Some(x) => panic!("Unknown secfunction: {}", x),
            None => panic!("Unknown secfunction index: {}", value.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bolt(pub bool);

impl From<(u16, &ValueMaps)> for Bolt {
    // "bolt": {0: "OFF", 1: "ON"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["bolt"].get(&value.0).map(String::as_str) {
            Some("ON") => Self(true),
            Some("OFF") => Self(false),
            Some(x) => panic!("Unknown state: {}", x),
            None => panic!("Unknown state index: {}", value.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stable(pub bool);

impl From<(u16, &ValueMaps)> for Stable {
    // "isstableflag": {1: "STABLE", 0: "UNSTABLE"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["isstableflag"].get(&value.0).map(String::as_str) {
            Some("STABLE") => Self(true),
            Some("UNSTABLE") => Self(false),
            Some(x) => panic!("Unknown stableflag: {}", x),
            None => panic!("Unknown stableflag index: {}", value.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutoRange(bool);

impl From<(u16, &ValueMaps)> for AutoRange {
    // "autorange": {1: "AUTO", 0: "MANUAL"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["autorange"].get(&value.0).map(String::as_str) {
            Some("AUTO") => Self(true),
            Some("MANUAL") => Self(false),
            Some(x) => panic!("Unknown autorange: {}", x),
            None => panic!("Unknown autorange index: {}", value.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modes(Vec<Mode>);

impl Modes {
    pub fn is(&self, mode: Mode) -> bool {
        self.0.contains(&mode)
    }
}

impl fmt::Display for Modes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = self
            .0
            .iter()
            .filter(|x| **x != Mode::None)
            .map(Mode::to_string)
            .collect::<Vec<String>>()
            .join(", ");
        f.write_str(&str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Mode {
    LowPassFilter,
    AutoSave,
    Calibration,
    None,
    Hold,
    AutoHold,
    MinMaxAvg,
    Record,
    Rel,
    RelPercent,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::LowPassFilter => f.write_str("Lowpass"),
            Mode::AutoSave => f.write_str("Autosave"),
            Mode::Calibration => f.write_str("Cal."),
            Mode::None => f.write_str("None"),
            Mode::Hold => f.write_str("Hold"),
            Mode::AutoHold => f.write_str("Autohold"),
            Mode::MinMaxAvg => f.write_str("Min/Max/Avg"),
            Mode::Record => f.write_str("Recording"),
            Mode::Rel => f.write_str("Rel."),
            Mode::RelPercent => f.write_str("Rel. %"),
        }
    }
}

impl From<(u16, &ValueMaps)> for Modes {
    // "mode": {8: "LOW_PASS_FILTER", 2: "AUTO_SAVE", 256: "CALIBRATION", 0: "NONE",
    //  4: "HOLD", 1: "AUTO_HOLD", 16: "MIN_MAX_AVG", 32: "RECORD", 64: "REL", 128: "REL_PERCENT"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;

        let mut modes = Vec::new();

        for (flag, name) in &maps["mode"] {
            if value.0 & *flag == *flag {
                let mode = match name.as_str() {
                    "LOW_PASS_FILTER" => Mode::LowPassFilter,
                    "AUTO_SAVE" => Mode::AutoSave,
                    "CALIBRATION" => Mode::Calibration,
                    "NONE" => Mode::None,
                    "HOLD" => Mode::Hold,
                    "AUTO_HOLD" => Mode::AutoHold,
                    "MIN_MAX_AVG" => Mode::MinMaxAvg,
                    "RECORD" => Mode::Record,
                    "REL" => Mode::Rel,
                    "REL_PERCENT" => Mode::RelPercent,
                    x => panic!("Unknown mode: {}", x),
                };
                modes.push(mode);
            }
        }
        Self(modes)
    }
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum State {
    Normal,
    Discharge,
    OL_Minus,
    Invalid,
    Blank,
    Inactive,
    OL,
    OpenTC,
}

impl From<(u16, &ValueMaps)> for State {
    // "state": {2: "NORMAL", 4: "DISCHARGE", 6: "OL_MINUS", 1: "INVALID", 3: "BLANK", 0: "INACTIVE", 5: "OL", 7: "OPEN_TC"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["state"].get(&value.0).map(String::as_str) {
            Some("NORMAL") => Self::Normal,
            Some("DISCHARGE") => Self::Discharge,
            Some("OL_MINUS") => Self::OL_Minus,
            Some("INVALID") => Self::Invalid,
            Some("BLANK") => Self::Blank,
            Some("INACTIVE") => Self::Inactive,
            Some("OL") => Self::OL,
            Some("OPEN_TC") => Self::OpenTC,
            Some(x) => panic!("Unknown state: {}", x),
            None => panic!("Unknown state index: {}", value.0),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum TransientState {
    Overload,
    RangeUp,
    NonT,
    OpenTC,
    RangeDown,
}

impl fmt::Display for TransientState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransientState::Overload => f.write_str("Overload"),
            TransientState::RangeUp => f.write_str("Range UP"),
            TransientState::NonT => f.write_str("NonT"),
            TransientState::OpenTC => f.write_str("Open Thermo element"),
            TransientState::RangeDown => f.write_str("Range DOWN"),
        }
    }
}

impl From<(u16, &ValueMaps)> for TransientState {
    // "transientstate": {3: "OVERLOAD", 1: "RANGE_UP", 0: "NON_T", 4: "OPEN_TC", 2: "RANGE_DOWN"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["transientstate"].get(&value.0).map(String::as_str) {
            Some("OVERLOAD") => Self::Overload,
            Some("RANGE_UP") => Self::RangeUp,
            Some("NON_T") => Self::NonT,
            Some("OPEN_TC") => Self::OpenTC,
            Some("RANGE_DOWN") => Self::RangeDown,
            Some(x) => panic!("Unknown state: {}", x),
            None => panic!("Unknown state index: {}", value.0),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum Attribute {
    LoOhms,
    ShortCircuit,
    OpenCircuit,
    GoodDiode,
    HighCurrent,
    //None,
    NegativeEdge,
    GlitchCircuit,
    PositiveEdge,
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                Attribute::LoOhms => f.write_str("LoΩ"),
                Attribute::ShortCircuit => f.write_str("ShortC"),
                Attribute::OpenCircuit => f.write_str("OpenC"),
                Attribute::GoodDiode => f.write_str("Diode OK"),
                Attribute::HighCurrent => f.write_str("High Current"),
                //Attribute::None => f.write_str("None"),
                //Attribute::None => Ok(()),
                Attribute::NegativeEdge => f.write_str("⬎"),
                Attribute::GlitchCircuit => f.write_str("Glitch Circuit"),
                Attribute::PositiveEdge => f.write_str("⬏"),
            }
        } else {
            match self {
                Attribute::LoOhms => f.write_str("LoOhm"),
                Attribute::ShortCircuit => f.write_str("Short Circuit"),
                Attribute::OpenCircuit => f.write_str("Open Circuit"),
                Attribute::GoodDiode => f.write_str("Good Diode"),
                Attribute::HighCurrent => f.write_str("High Current"),
                //Attribute::None => f.write_str("None"),
                //Attribute::None => Ok(()),
                Attribute::NegativeEdge => f.write_str("Negative Edge"),
                Attribute::GlitchCircuit => f.write_str("Glitch Circuit"),
                Attribute::PositiveEdge => f.write_str("Positive Edge"),
            }
        }
    }
}

impl TryFrom<(u16, &ValueMaps)> for Attribute {
    type Error = ();
    // "attribute": {5: "LO_OHMS", 2: "SHORT_CIRCUIT", 1: "OPEN_CIRCUIT", 4: "GOOD_DIODE",
    // 8: "HIGH_CURRENT", 0: "NONE", 6: "NEGATIVE_EDGE", 3: "GLITCH_CIRCUIT", 7: "POSITIVE_EDGE"}
    fn try_from(value: (u16, &ValueMaps)) -> std::result::Result<Self, Self::Error> {
        let maps = value.1;
        Ok(match maps["attribute"].get(&value.0).map(String::as_str) {
            Some("LO_OHMS") => Self::LoOhms,
            Some("SHORT_CIRCUIT") => Self::ShortCircuit,
            Some("OPEN_CIRCUIT") => Self::OpenCircuit,
            Some("GOOD_DIODE") => Self::GoodDiode,
            Some("HIGH_CURRENT") => Self::HighCurrent,
            Some("NONE") => return Err(()),
            Some("NEGATIVE_EDGE") => Self::NegativeEdge,
            Some("GLITCH_CIRCUIT") => Self::GlitchCircuit,
            Some("POSITIVE_EDGE") => Self::PositiveEdge,
            Some(x) => panic!("Unknown attribute: {}", x),
            None => panic!("Unknown attribute index: {}", value.0),
        })
    }
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum RecordType {
    Input,
    Interval,
}

impl From<(u16, &ValueMaps)> for RecordType {
    // "recordtype": {0: "INPUT", 1: "INTERVAL"}
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["recordtype"].get(&value.0).map(String::as_str) {
            Some("INPUT") => Self::Input,
            Some("INTERVAL") => Self::Interval,
            Some(x) => panic!("Unknown recordtype: {}", x),
            None => panic!("Unknown recordtype index: {}", value.0),
        }
    }
}

impl fmt::Display for RecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordType::Input => f.write_str("Input"),
            RecordType::Interval => f.write_str("Interval"),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub enum Unit {
    Farad,
    None,
    Percent,
    Seconds,
    AmpereAC,
    VoltAcPlusDc,
    CEL,
    dBV,
    dBm,
    dB,
    AmpereAcPlusDc,
    VoltDC,
    Volt,
    AmpereDC,
    VoltAC,
    Fahrenheit,
    Ohm,
    Siemens,
    Hertz,
    CrestFactor,
    Ampere,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Farad => f.write_str("F"),
            Unit::None => f.write_str(""),
            Unit::Percent => f.write_str("%"),
            Unit::Seconds => f.write_str("S"),
            Unit::AmpereAC => f.write_str("AAC"),
            Unit::VoltAcPlusDc => f.write_str("VAC+DC"),
            Unit::CEL => f.write_str("°C"),
            Unit::dBV => f.write_str("dBV"),
            Unit::dBm => f.write_str("dBm"),
            Unit::dB => f.write_str("db"),
            Unit::AmpereAcPlusDc => f.write_str("AAC+DC"),
            Unit::VoltDC => f.write_str("VDC"),
            Unit::Volt => f.write_str("V"),
            Unit::AmpereDC => f.write_str("ADC"),
            Unit::VoltAC => f.write_str("VAC"),
            Unit::Fahrenheit => f.write_str("°F"),
            Unit::Ohm => f.write_str("Ω"),
            Unit::Siemens => f.write_str("S"),
            Unit::Hertz => f.write_str("Hz"),
            Unit::CrestFactor => f.write_str("CF"),
            Unit::Ampere => f.write_str("A"),
        }
    }
}

impl From<(u16, &ValueMaps)> for Unit {
    // "unit": {15: "FAR", 0: "NONE", 16: "PCT", 12: "S", 6: "AAC", 3: "VAC_PLUS_DC",
    // 14: "CEL", 18: "dBV", 19: "dBm", 17: "dB", 7: "AAC_PLUS_DC", 1: "VDC", 4: "V",
    // 5: "ADC", 2: "VAC", 13: "F", 9: "OHM", 10: "SIE", 11: "Hz",
    // 20: "CREST_FACTOR", 8: "A"},
    fn from(value: (u16, &ValueMaps)) -> Self {
        let maps = value.1;
        match maps["unit"].get(&value.0).map(String::as_str) {
            Some("FAR") => Self::Fahrenheit,
            Some("NONE") => Self::None,
            Some("PCT") => Self::Percent,
            Some("S") => Self::Seconds, // ???
            Some("AAC") => Self::AmpereAC,
            Some("VAC_PLUS_DC") => Self::VoltAcPlusDc,
            Some("CEL") => Self::CEL,
            Some("dBV") => Self::dBV,
            Some("dBm") => Self::dBm,
            Some("dB") => Self::dB,
            Some("AAC_PLUS_DC") => Self::AmpereAcPlusDc,
            Some("VDC") => Self::VoltDC,
            Some("V") => Self::Volt,
            Some("ADC") => Self::AmpereDC,
            Some("VAC") => Self::VoltAC,
            Some("F") => Self::Farad,
            Some("OHM") => Self::Ohm,
            Some("SIE") => Self::Siemens,
            Some("Hz") => Self::Hertz,
            Some("CREST_FACTOR") => Self::CrestFactor,
            Some("A") => Self::Ampere,
            Some(x) => panic!("Unknown unit: {}", x),
            None => panic!("Unknown unit index: {}", value.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reading {
    pub reading_id: u16,
    pub value: f64,
    pub unit: Unit,
    pub unit_multiplier: i16,
    pub decimals: i16,
    pub display_digits: i16,
    pub state: State,
    pub attribute: Option<Attribute>,
    pub ts: DateTime<Utc>,
}

impl From<(RawReading, &ValueMaps)> for Reading {
    fn from(value: (RawReading, &ValueMaps)) -> Self {
        let maps = value.1;
        Self {
            reading_id: value.0.reading_id,
            value: value.0.value,
            unit: (value.0.unit, maps).into(),
            unit_multiplier: value.0.unit_multiplier,
            decimals: value.0.decimals,
            display_digits: value.0.display_digits,
            state: (value.0.state, maps).into(),
            attribute: (value.0.attribute, maps).try_into().ok(),
            ts: timestamp_to_datetime(value.0.ts),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub pri_function: PrimaryFunction,
    pub sec_function: SecondaryFunction,
    pub auto_range: AutoRange,
    pub unit: Unit,
    pub range_max: f64,
    pub unit_multiplier: i16,
    pub bolt: Bolt,
    pub ts: Option<DateTime<Utc>>,
    pub modes: Modes,
    pub readings: Vec<Reading>,
}

impl From<(RawMeasurement, &ValueMaps)> for Measurement {
    fn from(value: (RawMeasurement, &ValueMaps)) -> Self {
        let maps = value.1;

        let readings = value
            .0
            .readings
            .iter()
            .map(|rr| Reading::from((rr.clone(), maps)))
            .collect();

        Self {
            pri_function: (value.0.pri_function, maps).into(),
            sec_function: (value.0.sec_function, maps).into(),
            auto_range: (value.0.auto_range, maps).into(),
            unit: (value.0.unit, maps).into(),
            range_max: value.0.range_max,
            unit_multiplier: value.0.unit_multiplier,
            bolt: (value.0.bolt, maps).into(),
            ts: if value.0.ts as isize != 0 && value.0.ts.is_normal() {
                Some(timestamp_to_datetime(value.0.ts))
            } else {
                None
            },
            modes: (value.0.modes, maps).into(),
            readings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SavedMeasurement {
    pub seq_no: u16,
    pub pri_function: PrimaryFunction,
    pub sec_function: SecondaryFunction,
    pub auto_range: AutoRange,
    pub unit: Unit,
    pub range_max: f64,
    pub unit_multiplier: i16,
    pub bolt: Bolt,
    pub modes: Modes,
    pub readings: Vec<Reading>,
    pub name: String,
}

impl From<(RawSavedMeasurement, &ValueMaps)> for SavedMeasurement {
    fn from(value: (RawSavedMeasurement, &ValueMaps)) -> Self {
        let maps = value.1;

        let readings = value
            .0
            .readings
            .iter()
            .map(|rr| Reading::from((rr.clone(), maps)))
            .collect();

        Self {
            seq_no: value.0.seq_no,
            pri_function: (value.0.pri_function, maps).into(),
            sec_function: (value.0.sec_function, maps).into(),
            auto_range: (value.0.auto_range, maps).into(),
            unit: (value.0.unit, maps).into(),
            range_max: value.0.range_max,
            unit_multiplier: value.0.unit_multiplier,
            bolt: (value.0.bolt, maps).into(),
            modes: (value.0.modes, maps).into(),
            readings,
            name: value.0.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SavedMinMaxMeasurement {
    pub seq_no: u16,
    pub ts1: DateTime<Utc>,
    pub ts2: DateTime<Utc>,
    pub pri_function: PrimaryFunction,
    pub sec_function: SecondaryFunction,
    pub auto_range: AutoRange,
    pub unit: Unit,
    pub range_max: f64,
    pub unit_multiplier: i16,
    pub bolt: Bolt,
    pub ts3: DateTime<Utc>,
    pub modes: Modes,

    pub readings: Vec<Reading>,

    pub name: String,
}

impl From<(RawSavedMinMaxMeasurement, &ValueMaps)> for SavedMinMaxMeasurement {
    fn from(value: (RawSavedMinMaxMeasurement, &ValueMaps)) -> Self {
        let maps = value.1;

        let readings = value
            .0
            .readings
            .iter()
            .map(|rr| Reading::from((rr.clone(), maps)))
            .collect();

        Self {
            seq_no: value.0.seq_no,
            ts1: timestamp_to_datetime(value.0.ts1),
            ts2: timestamp_to_datetime(value.0.ts2),
            pri_function: (value.0.pri_function, maps).into(),
            sec_function: (value.0.sec_function, maps).into(),
            auto_range: (value.0.auto_range, maps).into(),
            unit: (value.0.unit, maps).into(),
            range_max: value.0.range_max,
            unit_multiplier: value.0.unit_multiplier,
            bolt: (value.0.bolt, maps).into(),
            ts3: timestamp_to_datetime(value.0.ts3),
            modes: (value.0.modes, maps).into(),
            readings,
            name: value.0.name,
        }
    }
}

pub type SavedPeakMeasurement = SavedMinMaxMeasurement;

#[derive(Debug, Clone)]
pub struct SavedRecordingSessionInfo {
    pub seq_no: u16,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub sample_interval: f64,
    pub event_threshold: f64,
    pub reading_index: u16,
    pub num_samples: u16,
    pub pri_function: PrimaryFunction,
    pub sec_function: SecondaryFunction,
    pub auto_range: AutoRange,
    pub unit: Unit,
    pub range_max: f64,
    pub unit_multiplier: i16,
    pub bolt: Bolt,
    pub modes: Modes,

    pub readings: Vec<Reading>,

    pub name: String,
}

impl From<(RawSavedRecordingSessionInfo, &ValueMaps)> for SavedRecordingSessionInfo {
    fn from(value: (RawSavedRecordingSessionInfo, &ValueMaps)) -> Self {
        let maps = value.1;

        let readings = value
            .0
            .readings
            .iter()
            .map(|rr| Reading::from((rr.clone(), maps)))
            .collect();

        Self {
            seq_no: value.0.seq_no,
            start_ts: timestamp_to_datetime(value.0.start_ts),
            end_ts: timestamp_to_datetime(value.0.end_ts),
            sample_interval: value.0.sample_interval,
            event_threshold: value.0.event_threshold,
            reading_index: value.0.reading_index,
            num_samples: value.0.num_samples,
            pri_function: (value.0.pri_function, maps).into(),
            sec_function: (value.0.sec_function, maps).into(),
            auto_range: (value.0.auto_range, maps).into(),
            unit: (value.0.unit, maps).into(),
            range_max: value.0.range_max,
            unit_multiplier: value.0.unit_multiplier,
            bolt: (value.0.bolt, maps).into(),
            modes: (value.0.modes, maps).into(),
            readings,
            name: value.0.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionRecordReadings {
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub span_readings: [Reading; 3], // max, min, avg (avg is the sum of all samples taken)
    pub sampling: u16,               // Count of samples accumulated in this reading
    pub fixed_reading: Reading,
    pub record_type: RecordType,
    pub stable: Stable,
    pub transient_state: TransientState,
}

impl TryFrom<(RawSessionRecordReadings, &ValueMaps)> for SessionRecordReadings {
    type Error = std::io::Error;
    fn try_from(
        value: (RawSessionRecordReadings, &ValueMaps),
    ) -> std::result::Result<Self, Self::Error> {
        let maps = value.1;

        let readings: Vec<Reading> = value
            .0
            .span_readings
            .iter()
            .map(|rr| Reading::from((rr.clone(), maps)))
            .collect();

        Ok(Self {
            start_ts: timestamp_to_datetime(value.0.start_ts),
            end_ts: timestamp_to_datetime(value.0.end_ts),
            span_readings: readings.try_into().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "readings must contain 3 readings",
                )
            })?,
            sampling: value.0.sampling,
            fixed_reading: Reading::from((value.0.fixed_reading.clone(), maps)),
            record_type: (value.0.record_type, maps).into(),
            stable: (value.0.stable, maps).into(),
            transient_state: (value.0.transient_state, maps).into(),
        })
    }
}

impl fmt::Display for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state {
            State::Normal => {
                assert!(!self.decimals.is_negative(), "Why should this negative?");
                let prec = f.precision().unwrap_or(self.decimals as usize);
                let width = f.width().unwrap_or(0);

                let v = self.value / (10_f64.powi(self.unit_multiplier as i32));
                let prefix = unit_prefix(self.unit_multiplier);

                f.write_fmt(format_args!("{:>width$.prec$} {}{}", v, prefix, self.unit))?;

                if f.alternate() {
                    if let Some(attr) = &self.attribute {
                        f.write_fmt(format_args!(" {:#}", attr))?;
                    }
                }
                Ok(())
            }
            State::Discharge => f.write_str("DISCHARGE"),
            State::OL_Minus => f.write_str("-OL"),
            State::Invalid => f.write_str("INVALID"),
            State::Blank => f.write_str("---"),
            State::Inactive => f.write_str("INACTIVE"),
            State::OL => f.write_str("OL"),
            State::OpenTC => f.write_str("OPEN-TC"),
        }
    }
}

pub enum Memory {
    Measurement(SavedMeasurement),
    MinMaxMeasurement(SavedMinMaxMeasurement),
    PeakMeasurement(SavedPeakMeasurement),
    Recording(SavedRecordingSessionInfo),
}

impl Memory {
    pub fn name(&self) -> &str {
        match self {
            Memory::Measurement(m) => m.name.as_ref(),
            Memory::MinMaxMeasurement(m) => m.name.as_ref(),
            Memory::PeakMeasurement(m) => m.name.as_ref(),
            Memory::Recording(m) => m.name.as_ref(),
        }
    }
}
