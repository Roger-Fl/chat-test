use bincode::{self, options};
pub use serde;
use std::ptr;
use std::mem::size_of;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::Cursor;
use std::error::Error;

#[derive(Debug)]
pub enum MsgError {
    DataSizeTooSmall { min_size: usize },
    Read(std::io::Error),
    Write(std::io::Error),
    Serialize(bincode::Error),
    Deserialize(bincode::Error),
    LogicError { msg: String },
}

impl std::fmt::Display for MsgError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            MsgError::DataSizeTooSmall{min_size} => {
                write!(f, "Expected at least {} bytes in buffer", min_size)
            },
            MsgError::Read(error) => {
                write!(f, "{}", error)
            },
            MsgError::Write(error) => {
                write!(f, "{}", error)
            },
            MsgError::Serialize(error) => {
                write!(f, "{}", error)
            },
            MsgError::Deserialize(error) => {
                write!(f, "{}", error)
            },
            MsgError::LogicError{msg} => {
                write!(f, "Logic error: {}", msg)
            },
        }
    }
}

impl Error for MsgError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

// fn write_buffer<T>(buffer: &mut [u8], index: usize, val: T) {
//     let ptr = &mut buffer[index] as *mut u8 as *mut T;
//     unsafe { ptr::write_unaligned(ptr, val); }
// }
// fn read_buffer<T>(buffer: &[u8], index: usize) -> T {
//     let ptr = &buffer[index] as *const u8 as *const T;
//     unsafe { ptr::read_unaligned(ptr) }
// }

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgInfo {
    // len: u32, need to write len to buffer first (not in serialized struct otherwise you cant know when the message is ready to be deserialized and parsed yet...)
    pub code: u32,
    data: Vec<u8>,
}

impl MsgInfo {
    pub fn decode_data<'a, T>(&'a self) -> Result<T, MsgError> 
    where
        T: ?Sized + serde::Deserialize<'a>
    {
        let res = bincode::deserialize::<'a, T>(self.data.as_slice());
        match res {
            Ok(v) => return Ok(v),
            Err(e) => Err(MsgError::Deserialize(e)),
        }
    }
}

pub struct MsgStream<'a> {
    pub msginfo: MsgInfo,
    pub buffer_rem: &'a [u8],
}

impl<'a> MsgStream<'a> {
    fn new(msginfo: MsgInfo, buffer_rem: &'a [u8]) -> Self {
        MsgStream {
            msginfo,
            buffer_rem
        }
    }
}

pub fn serialize_data<T>(code: u32, data: &T) -> Result<Vec<u8>, MsgError> 
where
    T: ?Sized + serde::Serialize,
{
    let data_encode = match bincode::serialize(&data) {
        Ok(v) => v,
        Err(e) => return Err(MsgError::Serialize(e)),
    };

    let msginfo = MsgInfo {
        code: code,
        data: data_encode,
    };

    let mut msg_data = match bincode::serialize(&msginfo) {
        Ok(v) => v,
        Err(e) => return Err(MsgError::Serialize(e)),
    };
    let msg_data_len = msg_data.len();

    let mut buffer = Vec::new();
    let sz = (msg_data_len) as u32;
    let res = buffer.write_u32::<LittleEndian>(sz);
    match res {
        Ok(_) => {},
        Err(e) => return Err(MsgError::Write(e)),
    }

    buffer.append(&mut msg_data);

    Ok(buffer)
}

fn msg_size(bytes: &[u8]) -> Result<u32, MsgError> {
    const MINSIZE: usize = size_of::<u32>();
    if bytes.len() < MINSIZE {
        return Err(MsgError::DataSizeTooSmall { min_size:MINSIZE });
    }
    let mut rdr = Cursor::new(bytes);
    let res = rdr.read_u32::<LittleEndian>();
    match res {
        Ok(sz) => Ok(sz),
        Err(e) => Err(MsgError::Read(e)),
    }
}

fn decode_msginfo(bytes: &[u8]) -> Result<MsgInfo, MsgError> {
    let res = bincode::deserialize(&bytes);
    match res {
        Ok(v) => return Ok(v),
        Err(e) => Err(MsgError::Deserialize(e)),
    }
}

pub fn parse_msgstream(buffer: &[u8]) -> Result<Option<MsgStream>, MsgError> {
    let sz = match msg_size(&buffer) {
        Ok(sz) => sz,
        Err(MsgError::DataSizeTooSmall { min_size }) => return Ok(None),
        Err(e) => return Err(e),
    } + size_of::<u32>() as u32; //for size

    if sz == 0 {
        return Err(MsgError::LogicError {msg: "0 sized msg detected".to_string()});
    }

    if (buffer.len() as u32) < sz {
        return Ok(None)
    }

    let newmsg_start = sz as usize;

    let msginfo = match decode_msginfo(&buffer[size_of::<u32>()..newmsg_start]) {
        Ok(msginfo) => msginfo,
        Err(e) => return Err(e),
    };

    let msgstream = MsgStream::new(msginfo, &buffer[newmsg_start..]);
    Ok(Some(msgstream))
}