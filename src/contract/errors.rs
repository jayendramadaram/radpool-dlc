use std::fmt;

#[derive(Debug, PartialEq)] 
pub enum ContractError {
    // contract errors
    MissingContractInfo,
    MissingOracles,
    InvalidThreshold(String),
    
    // descriptor errors
    MissingOutcomePayouts,
    MissingOracleNumericInfo,
    InvalidPayoutPoints(String),
    InvalidRoundingInterval(String),
}

impl std::error::Error for ContractError {}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractError::MissingContractInfo => {
                write!(f, "At least one contract info is required")
            }

            ContractError::MissingOracles => {
                write!(f, "At least one oracle is required")
            }
            
            ContractError::InvalidThreshold(msg) => {
                write!(f, "Invalid threshold value: {}", msg)
            }

            ContractError::MissingOutcomePayouts => {
                write!(f, "At least one outcome payout is required")
            }

            ContractError::MissingOracleNumericInfo => {
                write!(f, "oracle numeric info is required")
            }

            ContractError::InvalidPayoutPoints(msg) => {
                write!(f, "Invalid payout points: {}", msg)
            }

            ContractError::InvalidRoundingInterval(msg) => {
                write!(f, "Invalid rounding interval: {}", msg)
            }
        }
    }
}


pub type ContractResult<T> = Result<T, ContractError>;