use crate::error::WhoopError;

type Result<T> = std::result::Result<T, InvalidIndexError>;

pub struct InvalidIndexError;

pub trait BufferReader {
    fn read<const N: usize>(&mut self) -> Result<[u8; N]>;
    fn read_end<const N: usize>(&mut self) -> Result<[u8; N]>;
    fn pop_front(&mut self) -> Result<u8>;

    fn read_u32_le(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read()?))
    }
    fn read_u16_le(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read()?))
    }
}

impl BufferReader for Vec<u8> {
    fn read<const N: usize>(&mut self) -> Result<[u8; N]> {
        if self.len() < N {
            return Err(InvalidIndexError);
        }

        self.drain(0..N)
            .collect::<Vec<u8>>()
            .try_into()
            .map_err(|_| InvalidIndexError)
    }

    fn read_end<const N: usize>(&mut self) -> Result<[u8; N]> {
        let size = self.len();
        if size < N {
            return Err(InvalidIndexError);
        }

        self.drain((size - N)..size)
            .collect::<Vec<u8>>()
            .try_into()
            .map_err(|_| InvalidIndexError)
    }

    fn pop_front(&mut self) -> Result<u8> {
        if !self.is_empty() {
            Ok(self.remove(0))
        } else {
            Err(InvalidIndexError)
        }
    }
}

impl From<InvalidIndexError> for WhoopError {
    fn from(_: InvalidIndexError) -> Self {
        Self::InvalidIndexError
    }
}
