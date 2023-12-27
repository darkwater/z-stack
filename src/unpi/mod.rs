mod error;

use tokio_util::{
    bytes::{Buf, BufMut, Bytes, BytesMut},
    codec::{Decoder, Encoder},
};

use self::error::Error;

pub struct UnpiCodec;

impl Encoder<Message> for UnpiCodec {
    type Error = Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let raw = RawMessage::from(item);
        raw.write_to(dst);
        Ok(())
    }
}

impl Decoder for UnpiCodec {
    type Item = Message;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let Some(raw) = RawMessage::parse(src)? else {
            return Ok(None);
        };

        raw.verify()?;

        Ok(Some(raw.into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub cmd_type: CmdType,
    pub subsystem: Subsystem,
    pub command_id: u8,
    pub data: Bytes,
}

impl From<RawMessage> for Message {
    fn from(raw: RawMessage) -> Self {
        Self {
            cmd_type: raw.cmd_type().unwrap(),
            subsystem: raw.subsystem().unwrap(),
            command_id: raw.command_id(),
            data: raw.data,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RawMessage {
    subsystem: u8,
    command_id: u8,
    data: Bytes,
    check: u8,
}

impl RawMessage {
    pub fn parse(src: &mut BytesMut) -> Result<Option<Self>, Error> {
        if src.len() < 5 {
            return Ok(None);
        }
        let len = src[1];
        if src.len() < len as usize + 5 {
            return Ok(None);
        }

        let sof = src.get_u8();

        if sof != 0xfe {
            return Err(Error::InvalidSof(sof));
        }

        let len = src.get_u8();
        let subsystem = src.get_u8();
        let command_id = src.get_u8();

        let data = src.split_to(len as usize).freeze();

        let check = src.get_u8();

        Ok(Some(RawMessage { subsystem, command_id, data, check }))
    }

    pub fn write_to(&self, dst: &mut BytesMut) {
        dst.put_u8(0xfe);
        dst.put_u8(self.data.len() as u8);
        dst.put_u8(self.subsystem);
        dst.put_u8(self.command_id);
        dst.put(self.data.clone());
        dst.put_u8(self.check);
    }

    pub fn verify(&self) -> Result<(), Error> {
        let mut check = 0u8;
        check ^= self.subsystem;
        check ^= self.command_id;
        check ^= self.data.len() as u8;
        check = self.data.iter().fold(check, |acc, b| acc ^ b);
        check ^= self.check;

        if check == 0 {
            Ok(())
        } else {
            Err(Error::InvalidFcs(check))
        }
    }

    pub fn cmd_type(&self) -> Result<CmdType, Error> {
        CmdType::try_from(self.subsystem >> 5)
    }

    pub fn subsystem(&self) -> Result<Subsystem, Error> {
        Subsystem::try_from(self.subsystem & 0x1f)
    }

    pub fn command_id(&self) -> u8 {
        self.command_id
    }
}

impl From<Message> for RawMessage {
    fn from(msg: Message) -> Self {
        let mut raw = Self {
            subsystem: (msg.cmd_type as u8) << 5 | (msg.subsystem as u8),
            command_id: msg.command_id,
            data: msg.data,
            check: 0,
        };

        raw.check ^= raw.subsystem;
        raw.check ^= raw.command_id;
        raw.check ^= raw.data.len() as u8;
        raw.check = raw.data.iter().fold(raw.check, |acc, b| acc ^ b);

        raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CmdType {
    SyncRequest = 1,
    AsyncRequest = 2,
    SyncResponse = 3,
}

impl TryFrom<u8> for CmdType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(CmdType::SyncRequest),
            0x02 => Ok(CmdType::AsyncRequest),
            0x03 => Ok(CmdType::SyncResponse),
            _ => Err(Error::InvalidCmdType(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Subsystem {
    Sys = 0x01,
    Mac = 0x02,
    Nwk = 0x03,
    Af = 0x04,
    Zdo = 0x05,
    Sapi = 0x06,
    Util = 0x07,
    Debug = 0x08,
    App = 0x09,
    AppConfig = 0x0f,
    GreenPower = 0x15,
}

impl TryFrom<u8> for Subsystem {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Subsystem::Sys),
            0x02 => Ok(Subsystem::Mac),
            0x03 => Ok(Subsystem::Nwk),
            0x04 => Ok(Subsystem::Af),
            0x05 => Ok(Subsystem::Zdo),
            0x06 => Ok(Subsystem::Sapi),
            0x07 => Ok(Subsystem::Util),
            0x08 => Ok(Subsystem::Debug),
            0x09 => Ok(Subsystem::App),
            0x0f => Ok(Subsystem::AppConfig),
            0x15 => Ok(Subsystem::GreenPower),
            _ => Err(Error::InvalidSubsystem(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let msg = Message {
            cmd_type: CmdType::SyncRequest,
            subsystem: Subsystem::Sys,
            command_id: 0x01,
            data: Bytes::from_static(&[0x02, 0x04, 0x06, 0x08]),
        };

        let raw = RawMessage::from(msg.clone());

        let mut buf = BytesMut::new();
        raw.write_to(&mut buf);

        let decoded = RawMessage::parse(&mut buf).unwrap().unwrap();

        assert_eq!(raw, decoded);
    }
}
