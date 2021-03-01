use crate::{DayIndex, HourIndex, MinuteIndex, MonthIndex, SecondIndex, YearIndex};

impl From<u32> for YearIndex {
    fn from(data: u32) -> Self {
        YearIndex(data)
    }
}

impl Into<u32> for YearIndex {
    fn into(self) -> u32 {
        self.0
    }
}

impl From<u32> for MonthIndex {
    fn from(data: u32) -> Self {
        MonthIndex(data)
    }
}

impl Into<u32> for MonthIndex {
    fn into(self) -> u32 {
        self.0
    }
}

impl From<u32> for DayIndex {
    fn from(data: u32) -> Self {
        DayIndex(data)
    }
}

impl Into<u32> for DayIndex {
    fn into(self) -> u32 {
        self.0
    }
}

impl From<u32> for HourIndex {
    fn from(data: u32) -> Self {
        HourIndex(data)
    }
}

impl Into<u32> for HourIndex {
    fn into(self) -> u32 {
        self.0
    }
}

impl From<u32> for MinuteIndex {
    fn from(data: u32) -> Self {
        MinuteIndex(data)
    }
}

impl Into<u32> for MinuteIndex {
    fn into(self) -> u32 {
        self.0
    }
}

impl From<u32> for SecondIndex {
    fn from(data: u32) -> Self {
        SecondIndex(data)
    }
}

impl Into<u32> for SecondIndex {
    fn into(self) -> u32 {
        self.0
    }
}
