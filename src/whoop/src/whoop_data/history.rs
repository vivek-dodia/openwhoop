use chrono::NaiveDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryReading {
    pub unix: u32,
    pub bpm: u8,
    pub rr: Vec<u16>,
    pub activity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHistoryReading {
    pub time: NaiveDateTime,
    pub bpm: u8,
    pub rr: Vec<u16>,
    pub activity: Activity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum Activity {
    #[default]
    Unknown,
    Active,
    Inactive,
    Sleep,
    Awake,
}

impl From<i64> for Activity {
    fn from(value: i64) -> Self {
        match value {
            0..500_000_000 => Self::Inactive,
            500_000_000..1_000_000_000 => Self::Active,
            1_000_000_000..1_500_000_000 => Self::Sleep,
            1_500_000_000..=i64::MAX => Self::Awake,
            _ => {
                println!("{}, {}", value, u64::from_le_bytes(value.to_le_bytes()));
                Self::Unknown
            }
        }
    }
}
