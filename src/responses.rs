//! Responses to commands

/// Response to `get_fw_version()`
#[derive(Debug)]
pub struct FwVersion {
    /// Major version
    pub major: u8,
    /// Minor version
    pub minor: u8,
    /// Hardware version
    pub hw: [u8; 10],
    /// 96 bit ID of MCU
    pub uuid: [u8; 12],
}

/// Response to `get_values()`
#[derive(Debug)]
pub struct Values {
    /// FET temperature in C
    pub temp_fet: f32,
    /// Motor temperature in C
    pub temp_motor: f32,
    /// Motor current in A
    pub motor_current: f32,
    /// Input current in A
    pub input_current: f32,
    /// ?
    pub id: f32,
    /// ?
    pub iq: f32,
    /// Motor duty cycle
    pub duty_cycle: f32,
    /// Motor RPM
    pub rpm: f32,
    /// Input voltage in V
    pub input_voltage: f32,
    /// Amp hours drawn in Ah
    pub amp_hours: f32,
    /// Amp hours charged in Ah
    pub amp_hours_charged: f32,
    /// Watt hours drawn in Wh
    pub watt_hours: f32,
    /// Watt hours charged in Wh
    pub watt_hours_charged: f32,
    /// Motor tachometer
    pub tachometer: u32,
    /// Absolute reading of motor tachometer
    pub tachometer_abs: u32,
    /// Fault state of controller
    pub fault: Fault,
    /// Motor position ?
    pub pid_pos: f32,
    /// ID of controller
    pub controller_id: u8,
}

/// Controller faults
#[derive(Debug)]
pub enum Fault {
    /// No faults
    None,
    /// Input voltage too high
    OverVoltage,
    /// Input voltage too low
    UnderVoltage,
    /// DRV error
    Drv,
    /// Current too high
    AbsOverCurrent,
    /// FET temperature too high
    OverTempFet,
    /// Motor temperature too high
    OverTempMotor,
}
