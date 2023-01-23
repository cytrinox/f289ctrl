use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Command {
    Id,
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
    // Clock
    GetClock,
    SetClock(u64),
}
