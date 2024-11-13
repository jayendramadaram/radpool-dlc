use dlc_manager::error::Error as DlcManagerError;
use std::fmt;

#[derive(Debug)]
pub enum ContractError {
    // contract errors
    MissingContractInfo,
    MissingOracles,
    InvalidThreshold,

    // descriptor errors
    MissingOutcomePayouts,
    MissingOracleNumericInfo,
    InvalidPayoutPoints,
    InvalidRoundingInterval,
    InvalidPayoutFunctionPieceSequence,

    // Wrapped error from dlc-manager crate
    DlcManagerError(DlcManagerError),
}

impl std::error::Error for ContractError {}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingContractInfo => {
                write!(f, "At least one contract info is required")
            }

            Self::MissingOracles => {
                write!(f, "At least one oracle is required")
            }

            Self::InvalidThreshold => {
                write!(f, "Threshold is out of range or is zero")
            }

            Self::MissingOutcomePayouts => {
                write!(f, "At least one outcome payout is required")
            }

            Self::MissingOracleNumericInfo => {
                write!(f, "oracle numeric info is required")
            }

            Self::InvalidPayoutPoints => {
                write!(f, "at most more than one payout point is required")
            }

            Self::InvalidRoundingInterval => {
                write!(f, "at least one rounding interval is required")
            }

            Self::InvalidPayoutFunctionPieceSequence => {
                write!(f, "Invalid payout function piece sequence")
            }

            Self::DlcManagerError(e) => e.fmt(f),
        }
    }
}

impl From<DlcManagerError> for ContractError {
    fn from(error: DlcManagerError) -> Self {
        Self::DlcManagerError(error)
    }
}

pub type ContractResult<T> = Result<T, ContractError>;
