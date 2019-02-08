use embedded_hal::serial::{Read, Write};
use serialport::prelude::*;
use vesc_comm::VescConnection;

fn main() {
    let (port1, port2) = {
        let mut port = serialport::open("/dev/tty.usbmodem301").unwrap();
        port.set_baud_rate(115200).unwrap();
        (Port::new(port.try_clone().unwrap()), Port::new(port))
    };

    let mut conn = VescConnection::new(port1, port2);

    dbg!(conn.get_fw_version()).ok();
    dbg!(conn.get_values()).ok();
    dbg!(conn.set_current(100_000u32)).ok();
}

struct Port {
    inner: Box<SerialPort>,
}

impl Port {
    fn new(inner: Box<SerialPort>) -> Self {
        Port { inner }
    }
}

impl Read<u8> for Port {
    type Error = std::io::Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let mut buf = [0u8];
        match self.inner.read(&mut buf) {
            Ok(1) => Ok(buf[0]),
            Ok(_) => Err(nb::Error::Other(std::io::Error::new(
                std::io::ErrorKind::Other,
                "read wrong number of bytes",
            ))),
            Err(e) => Err(nb::Error::Other(e)),
        }
    }
}

impl Write<u8> for Port {
    type Error = std::io::Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        match self.inner.write(&[word]) {
            Ok(1) => Ok(()),
            Ok(_) => Err(nb::Error::Other(std::io::Error::new(
                std::io::ErrorKind::Other,
                "wrote wrong number of bytes",
            ))),
            Err(e) => Err(nb::Error::Other(e)),
        }
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        Ok(())
    }
}
