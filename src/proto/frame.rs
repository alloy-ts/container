use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) const VERSION: u8 = 1;
pub(crate) const HEADER_LEN: usize = 16;
pub(crate) const MAX_PAYLOAD: usize = 8 * 1024 * 1024;
/// `flags` bit set on a stdin frame that closes the write side.
pub(crate) const FLAG_EOF: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum FrameKind {
    Request = 1,
    Response = 2,
    Stdin = 3,
    Stdout = 4,
    Stderr = 5,
    Exit = 6,
    Resize = 7,
}

impl FrameKind {
    pub(crate) fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::Request,
            2 => Self::Response,
            3 => Self::Stdin,
            4 => Self::Stdout,
            5 => Self::Stderr,
            6 => Self::Exit,
            7 => Self::Resize,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frame {
    pub kind: FrameKind,
    pub flags: u16,
    pub id: u32,
    pub payload: Vec<u8>,
}

impl Frame {
    pub(crate) fn request(id: u32, payload: Vec<u8>) -> Self {
        Self {
            kind: FrameKind::Request,
            flags: 0,
            id,
            payload,
        }
    }

    pub(crate) fn response(id: u32, payload: Vec<u8>) -> Self {
        Self {
            kind: FrameKind::Response,
            flags: 0,
            id,
            payload,
        }
    }

    pub(crate) fn stdio(kind: FrameKind, pid: u32, payload: Vec<u8>) -> Self {
        Self {
            kind,
            flags: 0,
            id: pid,
            payload,
        }
    }

    pub(crate) fn stdin_eof(pid: u32) -> Self {
        Self {
            kind: FrameKind::Stdin,
            flags: FLAG_EOF,
            id: pid,
            payload: Vec::new(),
        }
    }

    pub(crate) fn exit(pid: u32, code: i32, signal: u16) -> Self {
        Self {
            kind: FrameKind::Exit,
            flags: signal,
            id: pid,
            payload: code.to_le_bytes().to_vec(),
        }
    }

    pub(crate) async fn write_to<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        if self.payload.len() > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "frame payload exceeds 8MiB",
            ));
        }
        let mut header = [0u8; HEADER_LEN];
        header[0..4].copy_from_slice(b"CTNR");
        header[4] = VERSION;
        header[5] = self.kind as u8;
        header[6..8].copy_from_slice(&self.flags.to_le_bytes());
        header[8..12].copy_from_slice(&self.id.to_le_bytes());
        header[12..16].copy_from_slice(&(self.payload.len() as u32).to_le_bytes());
        writer.write_all(&header).await?;
        writer.write_all(&self.payload).await?;
        Ok(())
    }

    pub(crate) async fn read_from<R>(reader: &mut R) -> io::Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let mut header = [0u8; HEADER_LEN];
        reader.read_exact(&mut header).await?;
        if &header[0..4] != b"CTNR" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "bad frame magic",
            ));
        }
        if header[4] != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported frame version {}", header[4]),
            ));
        }
        let kind = FrameKind::from_u8(header[5]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown frame kind {}", header[5]),
            )
        })?;
        let flags = u16::from_le_bytes([header[6], header[7]]);
        let id = u32::from_le_bytes(header[8..12].try_into().unwrap());
        let len = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
        if len > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame payload exceeds 8MiB",
            ));
        }
        let mut payload = vec![0u8; len];
        if len > 0 {
            reader.read_exact(&mut payload).await?;
        }
        Ok(Self {
            kind,
            flags,
            id,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip_every_kind() {
        let frames = vec![
            Frame::request(7, br#"{"op":"state"}"#.to_vec()),
            Frame::response(7, br#"{"op":"ok"}"#.to_vec()),
            Frame::stdio(FrameKind::Stdout, 42, b"hello".to_vec()),
            Frame::stdio(FrameKind::Stderr, 42, b"warn".to_vec()),
            Frame::stdin_eof(42),
            Frame::exit(42, 0, 0),
            Frame {
                kind: FrameKind::Resize,
                flags: 0,
                id: 42,
                payload: vec![24, 0, 80, 0],
            },
        ];
        let (mut client, mut server) = tokio::io::duplex(4096);
        let expected = frames.clone();
        let write = async {
            for frame in frames {
                frame.write_to(&mut client).await.unwrap();
            }
        };
        let read = async {
            let mut got = Vec::new();
            for _ in 0..expected.len() {
                got.push(Frame::read_from(&mut server).await.unwrap());
            }
            got
        };
        let (_, got) = tokio::join!(write, read);
        assert_eq!(got, expected);
    }

    #[tokio::test]
    async fn rejects_bad_magic() {
        let (mut client, mut server) = tokio::io::duplex(64);
        let send = async {
            client.write_all(b"NOPE").await.unwrap();
            client.write_all(&[0u8; 12]).await.unwrap();
        };
        let recv = async { Frame::read_from(&mut server).await };
        let (_, err) = tokio::join!(send, recv);
        assert!(err.unwrap_err().to_string().contains("magic"));
    }
}
