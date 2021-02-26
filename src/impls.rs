use crate::{DayIndex, HourIndex, MinuteIndex, MonthIndex, SecondIndex, YearIndex};

impl From<u32> for YearIndex {
    fn from(data: u32) -> Self {
        YearIndex(data as i32)
    }
}

impl From<u32> for MonthIndex {
    fn from(data: u32) -> Self {
        MonthIndex(data as i32)
    }
}

impl From<u32> for DayIndex {
    fn from(data: u32) -> Self {
        DayIndex(data)
    }
}

impl From<u32> for HourIndex {
    fn from(data: u32) -> Self {
        HourIndex(data)
    }
}

impl From<u32> for MinuteIndex {
    fn from(data: u32) -> Self {
        MinuteIndex(data)
    }
}

impl From<u32> for SecondIndex {
    fn from(data: u32) -> Self {
        SecondIndex(data)
    }
}
