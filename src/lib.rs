//! VESC communication library

#![no_std]
#![deny(missing_docs)]

#[macro_use(block)]
extern crate nb;

use byteorder::{BigEndian, ByteOrder};
use embedded_hal::serial::{Read, Write};
use failure::Fail;
use heapless::{consts::U128, Vec};

pub mod responses;

/// Connection to a VESC
pub struct VescConnection<R, W> {
    r: R,
    w: W,
}

impl<R: Read<u8>, W: Write<u8>> VescConnection<R, W> {
    /// Open a new connection with a VESC, currenly using embedded-hal Serial `Read` and `Write` traits
    pub fn new(r: R, w: W) -> Self {
        VescConnection { r, w }
    }

    /// Send a command over a connection, might have a response (Need to improve this)
    pub fn get_fw_version(&mut self) -> nb::Result<responses::FwVersion, Error> {
        write_packet(&[Command::FwVersion.value()], &mut self.w)?;

        let payload = read_packet(&mut self.r)?;

        if payload[0] != Command::FwVersion.value() {
            return Err(nb::Error::Other(Error::ParseError));
        }

        let mut uuid = [0u8; 12];
        for i in 0..12 {
            uuid[i] = payload[payload.len() - 12 + i];
        }

        // Longest currently defined HW_NAME is 10 characters
        // No fixed length hence ugly reading
        let mut hw = [0u8; 10];
        for i in 0..(payload.len() - 15) {
            hw[i] = payload[3 + i];
        }

        Ok(responses::FwVersion {
            major: payload[1],
            minor: payload[2],
            hw,
            uuid,
        })
    }

    /// Gets various sensor data from the VESC
    pub fn get_values(&mut self) -> nb::Result<responses::Values, Error> {
        write_packet(&[Command::GetValues.value()], &mut self.w)?;

        let payload = read_packet(&mut self.r)?;

        if payload[0] != Command::GetValues.value() {
            return Err(nb::Error::Other(Error::ParseError));
        }

        Ok(responses::Values {
            temp_fet: f32::from(BigEndian::read_u16(&payload[1..3])) / 10.0,
            temp_motor: f32::from(BigEndian::read_u16(&payload[3..5])) / 10.0,
            motor_current: BigEndian::read_u32(&payload[5..9]) as f32 / 100.0,
            input_current: BigEndian::read_u32(&payload[9..13]) as f32 / 100.0,
            id: BigEndian::read_u32(&payload[13..17]) as f32 / 100.0,
            iq: BigEndian::read_u32(&payload[17..21]) as f32 / 100.0,
            duty_cycle: f32::from(BigEndian::read_u16(&payload[21..23])) / 1_000.0,
            rpm: BigEndian::read_u32(&payload[23..27]) as f32,
            input_voltage: f32::from(BigEndian::read_u16(&payload[27..29])) / 10.0,
            amp_hours: BigEndian::read_u32(&payload[29..33]) as f32 / 10_000.0,
            amp_hours_charged: BigEndian::read_u32(&payload[33..37]) as f32 / 10_000.0,
            watt_hours: BigEndian::read_u32(&payload[37..41]) as f32 / 10_000.0,
            watt_hours_charged: BigEndian::read_u32(&payload[41..45]) as f32 / 10_000.0,
            tachometer: BigEndian::read_u32(&payload[45..49]),
            tachometer_abs: BigEndian::read_u32(&payload[49..53]),
            fault: responses::Fault::from_u8(payload[53]).unwrap(),
            pid_pos: BigEndian::read_u32(&payload[54..58]) as f32 / 1_000_000.0,
            controller_id: 0,
        })
    }
}

// Constructs a packet from a payload (adds start/stop bytes, length and CRC)
fn write_packet<W: Write<u8>>(payload: &[u8], w: &mut W) -> nb::Result<(), Error> {
    let hash = crc(&payload);

    // 2 for short packets and 3 for long packets
    block!(w.write(0x02)).ok();

    // If payload.len() > 255, then start byte should be 3
    // and the next two should be the length
    block!(w.write(payload.len() as u8)).ok();

    for byte in payload {
        block!(w.write(*byte)).ok();
    }

    // Always CRC16
    for byte in &hash {
        block!(w.write(*byte)).ok();
    }

    // Stop byte
    block!(w.write(0x03)).ok();

    Ok(())
}

// Reads a packet, checks it and returns it's payload
fn read_packet<R: Read<u8>>(r: &mut R) -> nb::Result<Vec<u8, U128>, Error> {
    let mut payload = Vec::new();

    // Read correct number of bytes into payload
    {
        let payload_len: usize = match block!(r.read()).ok().unwrap() {
            0x02 => block!(r.read()).ok().unwrap().into(),
            0x03 => {
                let mut buf = [0u8; 2];
                buf[0] = block!(r.read()).ok().unwrap();
                buf[1] = block!(r.read()).ok().unwrap();

                BigEndian::read_u16(&buf).into()
            }
            _ => {
                return Err(nb::Error::Other(Error::IoError));
            }
        };

        for _ in 0..payload_len {
            payload.push(block!(r.read()).ok().unwrap()).unwrap();
        }
    }

    // Check CRC
    {
        let calculated_hash = crc(&payload);

        let read_hash = {
            let mut hash: [u8; 2] = [0; 2];

            hash[0] = block!(r.read()).ok().unwrap();
            hash[1] = block!(r.read()).ok().unwrap();

            hash
        };

        if calculated_hash != read_hash {
            return Err(nb::Error::Other(Error::ChecksumError));
        }
    }

    // Sanity check that the last byte is the stop byte
    if block!(r.read()).ok().unwrap() != 0x03 {
        return Err(nb::Error::Other(Error::ParseError));
    }

    Ok(payload)
}

fn crc(payload: &[u8]) -> [u8; 2] {
    let mut hash: [u8; 2] = [0; 2];
    BigEndian::write_u16(&mut hash, crc16::State::<crc16::XMODEM>::calculate(&payload));

    hash
}

/// Errors returned if a command fails
#[derive(Fail, Debug)]
pub enum Error {
    /// Error occured during IO
    #[fail(display = "Error occured during IO")]
    IoError,
    /// Checksum mismatch
    #[fail(display = "Checksum mismatch")]
    ChecksumError,
    /// Error occured during parsing
    #[fail(display = "Error occured during parsing")]
    ParseError,
}

#[allow(dead_code)]
#[derive(Debug)]
enum Command {
    FwVersion,
    JumpToBootloader,
    EraseNewApp,
    WriteNewAppData,
    GetValues,
    SetDuty,
    SetCurrent,
    SetCurrentBrake,
    SetRpm,
    SetPos,
    SetHandbrake,
    SetDetect,
    SetServoPos,
    SetMcConf,
    GetMcConf,
    GetMcConfDefault,
    SetAppConf,
    GetAppConf,
    GetAppConfDefault,
    SamplePrint,
    TerminalCmd,
    DetectMotorParam,
    DetectMotorRL,
    DetectMotorFluxLinkage,
    DetectEncoder,
    DetectHallFoc,
    Reboot,
    Alive,
    GetDecodedPpm,
    GetDecodedAdc,
    GetDecodedChuck,
    ForwardCan,
    SetChuckData,
    CustomAppData,
    NrfStartPairing,
}

impl Command {
    fn value(&self) -> u8 {
        match *self {
            Command::FwVersion => 0,
            Command::JumpToBootloader => 1,
            Command::EraseNewApp => 2,
            Command::WriteNewAppData => 3,
            Command::GetValues => 4,
            Command::SetDuty => 5,
            Command::SetCurrent => 6,
            Command::SetCurrentBrake => 7,
            Command::SetRpm => 8,
            Command::SetPos => 9,
            Command::SetHandbrake => 10,
            Command::SetDetect => 11,
            Command::SetServoPos => 12,
            Command::SetMcConf => 13,
            Command::GetMcConf => 14,
            Command::GetMcConfDefault => 15,
            Command::SetAppConf => 16,
            Command::GetAppConf => 17,
            Command::GetAppConfDefault => 18,
            Command::SamplePrint => 19,
            Command::TerminalCmd => 20,
            Command::DetectMotorParam => 21,
            Command::DetectMotorRL => 22,
            Command::DetectMotorFluxLinkage => 23,
            Command::DetectEncoder => 24,
            Command::DetectHallFoc => 25,
            Command::Reboot => 26,
            Command::Alive => 27,
            Command::GetDecodedPpm => 28,
            Command::GetDecodedAdc => 29,
            Command::GetDecodedChuck => 30,
            Command::ForwardCan => 31,
            Command::SetChuckData => 32,
            Command::CustomAppData => 33,
            Command::NrfStartPairing => 34,
        }
    }
}

impl responses::Fault {
    /*
    fn value(&self) -> u8 {
        match *self {
            responses::Fault::None => 0,
            responses::Fault::OverVoltage => 1,
            responses::Fault::UnderVoltage => 2,
            responses::Fault::Drv => 3,
            responses::Fault::AbsOverCurrent => 4,
            responses::Fault::OverTempFet => 5,
            responses::Fault::OverTempMotor => 6,
        }
    }
    */

    fn from_u8(n: u8) -> Result<Self, crate::Error> {
        match n {
            0 => Ok(responses::Fault::None),
            1 => Ok(responses::Fault::OverVoltage),
            2 => Ok(responses::Fault::UnderVoltage),
            3 => Ok(responses::Fault::Drv),
            4 => Ok(responses::Fault::AbsOverCurrent),
            5 => Ok(responses::Fault::OverTempFet),
            6 => Ok(responses::Fault::OverTempMotor),
            _ => Err(crate::Error::ParseError),
        }
    }
}
