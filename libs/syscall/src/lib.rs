#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![no_std]

use bitbybit::bitenum;

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    tests.iter().for_each(|test| test());
}

#[derive(Debug, PartialEq, Eq)]
pub enum SystemCallError {
    Invalid(u64),
}

#[derive(Debug, PartialEq, Eq)]
#[bitenum(u64)]
pub enum SystemCall {
    Exit = 0,
    Sleep = 1,
    SleepUntilMessageReceived = 2,
    IdentityMap = 3,
    SendMessage = 4,
    ReceiveMessage = 5,
    RegisterServer = 6,

    // TODO: Remove these
    Spawn = 7,
    Allocate = 8,
    Deallocate = 9,
    DurationSinceBootup = 10,
}

impl TryFrom<u64> for SystemCall {
    type Error = SystemCallError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new_with_raw_value(value).map_err(SystemCallError::Invalid)
    }
}
