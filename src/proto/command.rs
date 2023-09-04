use std::{fmt::Display, time::Duration};

#[derive(Debug, Copy, Clone)]
pub enum ClearMemory {
    All,
    Measurements,
    MinMax,
    Peak,
    Recordings,
}

#[derive(Debug, Copy, Clone)]
pub enum DezibelReference {
    Ref4,
    Ref8,
    Ref16,
    Ref25,
    Ref32,
    Ref50,
    Ref75,
    Ref600,
    Ref1000,
    Custom,
}

impl Display for DezibelReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DezibelReference::Ref4 => f.write_str("4"),
            DezibelReference::Ref8 => f.write_str("8"),
            DezibelReference::Ref16 => f.write_str("16"),
            DezibelReference::Ref25 => f.write_str("25"),
            DezibelReference::Ref32 => f.write_str("32"),
            DezibelReference::Ref50 => f.write_str("50"),
            DezibelReference::Ref75 => f.write_str("75"),
            DezibelReference::Ref600 => f.write_str("600"),
            DezibelReference::Ref1000 => f.write_str("1000"),
            DezibelReference::Custom => f.write_str("CUSTOM"),
        }
    }
}

impl clap::ValueEnum for DezibelReference {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Ref4,
            Self::Ref8,
            Self::Ref16,
            Self::Ref25,
            Self::Ref32,
            Self::Ref50,
            Self::Ref75,
            Self::Ref600,
            Self::Ref1000,
            Self::Custom,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            DezibelReference::Ref4 => clap::builder::PossibleValue::new("4"),
            DezibelReference::Ref8 => clap::builder::PossibleValue::new("8"),
            DezibelReference::Ref16 => clap::builder::PossibleValue::new("16"),
            DezibelReference::Ref25 => clap::builder::PossibleValue::new("25"),
            DezibelReference::Ref32 => clap::builder::PossibleValue::new("32"),
            DezibelReference::Ref50 => clap::builder::PossibleValue::new("50"),
            DezibelReference::Ref75 => clap::builder::PossibleValue::new("75"),
            DezibelReference::Ref600 => clap::builder::PossibleValue::new("600"),
            DezibelReference::Ref1000 => clap::builder::PossibleValue::new("1000"),
            DezibelReference::Custom => clap::builder::PossibleValue::new("CUSTOM"),
        })
    }
}

impl clap::ValueEnum for ClearMemory {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Measurements,
            Self::MinMax,
            Self::Peak,
            Self::Recordings,
            Self::All,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::All => clap::builder::PossibleValue::new("all"),
            Self::Measurements => clap::builder::PossibleValue::new("measurements"),
            Self::MinMax => clap::builder::PossibleValue::new("minmax"),
            Self::Peak => clap::builder::PossibleValue::new("peak"),
            Self::Recordings => clap::builder::PossibleValue::new("recordings"),
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DigitCount {
    Digit4,
    Digit5,
}

impl clap::ValueEnum for DigitCount {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Digit4, Self::Digit5]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Digit4 => clap::builder::PossibleValue::new("4"),
            Self::Digit5 => clap::builder::PossibleValue::new("5"),
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Language {
    German,
    English,
    French,
    Italian,
    Spanish,
    Japanese,
    Chinese,
}

impl clap::ValueEnum for Language {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::German, Self::English]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::German => clap::builder::PossibleValue::new("GERMAN"),
            Self::English => clap::builder::PossibleValue::new("ENGLISH"),
            Self::French => clap::builder::PossibleValue::new("FRENCH"),
            Self::Italian => clap::builder::PossibleValue::new("ITALIAN"),
            Self::Spanish => clap::builder::PossibleValue::new("SPANISH"),
            Self::Japanese => clap::builder::PossibleValue::new("JAPANESE"),
            Self::Chinese => clap::builder::PossibleValue::new("CHINESE"),
        })
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum DateFormat {
    DD_MM,
    MM_DD,
}

impl clap::ValueEnum for DateFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::DD_MM, Self::MM_DD]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::DD_MM => clap::builder::PossibleValue::new("dd/mm"),
            Self::MM_DD => clap::builder::PossibleValue::new("mm/dd"),
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TimeFormat {
    Time12,
    Time24,
}

impl clap::ValueEnum for TimeFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Time12, Self::Time24]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Time12 => clap::builder::PossibleValue::new("12"),
            Self::Time24 => clap::builder::PossibleValue::new("24"),
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum NumericFormat {
    Point,
    Comma,
}

impl clap::ValueEnum for NumericFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Point, Self::Comma]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Point => clap::builder::PossibleValue::new("POINT"),
            Self::Comma => clap::builder::PossibleValue::new("COMMA"),
        })
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Id,
    // Maps
    QueryMap(String),
    // Backlight
    SetBacklightTimeout(Duration),
    GetBacklightTimeout,
    // Auto power off
    SetDevicePowerOff(Duration),
    GetDevicePowerOff,
    // Owner info
    GetOperator,
    SetOperator(String),
    GetCompany,
    SetCompany(String),
    GetSite,
    SetSite(String),
    GetContact,
    SetContact(String),
    // Beeper
    GetBeeper,
    SetBeeper(bool),
    // Smoothing
    GetSmoothing,
    SetSmoothing(bool),
    // Clock
    GetClock,
    SetClock(u64),
    // Memory slot names
    GetSaveName(u16),
    SetSaveName(u16, String),
    // Measurements
    GetMemoryStat,
    GetMeasurementBinary,
    QuerySavedMeasurement(usize),
    QueryMinMaxSessionInfo(usize),
    QueryPeakSessionInfo(usize),
    QueryRecordedSessionInfo(usize),
    QuerySessionRecordReadings(usize, usize),
    Clear(ClearMemory),

    ResetDevice,
    GetCustomDbm,
    SetCustomDbm(u16),
    GetDigitCount,
    SetDigitCount(DigitCount),

    GetAutoHoldEventThreshold,
    SetAutoHoldEventThreshold(u8),

    GetRecordingEventThreshold,
    SetRecordingEventThreshold(u8),

    GetLanguage,
    SetLanguage(Language),

    GetDateFormat,
    SetDateFormat(DateFormat),

    GetTimeFormat,
    SetTimeFormat(TimeFormat),

    GetNumFormat,
    SetNumFormat(NumericFormat),

    GetDbmRef,
    SetDbmRef(DezibelReference),

    GetTempOffset,
    SetTempOffset(i16),
}
