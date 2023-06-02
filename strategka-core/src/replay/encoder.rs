use log::warn;

use super::error::Result;
use std::any::type_name;
use std::io::ErrorKind;
use std::io::Write;

pub fn encode_be_u32<'a, W: Write>(value: u32, mut sink: W) -> Result<'a, ()> {
    let mut buff: [u8; 4] = [0; 4];
    buff.copy_from_slice(&value.to_be_bytes());
    sink.write_all(&buff)?;
    Ok(())
}

pub fn length_encoded<'a, W: Write, F>(mut sink: W, body: F) -> Result<'a, ()>
where
    F: FnOnce(&mut Vec<u8>) -> Result<'a, ()>,
{
    let mut buff = vec![];
    body(&mut buff)?;
    encode_be_u32(buff.len() as u32, &mut sink)?;
    if !buff.is_empty() {
        sink.write_all(&buff)?;
    }
    Ok(())
}


/// Guard that allow to write 0 bytes with ciborium to buffers but warns about that.
pub fn ciborium_into_writer<'a, T: ?Sized + serde::Serialize, W: Write>(
    value: &T, 
    writer: W,
) -> Result<'a, ()> {
    match ciborium::into_writer(value, writer) {
        Err(ciborium::ser::Error::Io(e)) => match e.kind() {
            ErrorKind::WriteZero => {
                warn!("Serialization body of {} is empty!", type_name::<T>());
                Ok(())
            }
            _ => Err(ciborium::ser::Error::Io(e).into()),
        },
        Err(e) => Err(e.into()),
        Ok(_) => Ok(()),
    }
}
