use steel::*;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum BlackgoldStakeError {
    #[error("No deposits")]
    NoDeposits = 0,

    #[error("Insufficient reserves")]
    InsufficientReserves = 1,
}

error!(BlackgoldStakeError);
