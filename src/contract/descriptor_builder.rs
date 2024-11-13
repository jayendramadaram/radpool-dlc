use std::collections::HashMap;

use dlc::{EnumerationPayout, Payout};
use dlc_manager::{
    contract::{
        enum_descriptor::EnumDescriptor,
        numerical_descriptor::{DifferenceParams, NumericalDescriptor},
    },
    payout_curve::{
        PayoutFunction, PayoutFunctionPiece, PayoutPoint, PolynomialPayoutCurvePiece,
        RoundingInterval, RoundingIntervals,
    },
};
use dlc_trie::OracleNumericInfo;

use super::errors::{ContractError, ContractResult};

#[derive(Default)]
/// Builder for `EnumDescriptor`
pub struct EnumDescriptorBuilder {
    outcome_payouts: Vec<EnumerationPayout>,
}

impl EnumDescriptorBuilder {
    /// Returns a new `EnumDescriptorBuilder` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_payout(mut self, outcome: String, offer: u64, accept: u64) -> Self {
        let enum_payout = EnumerationPayout {
            outcome,
            payout: Payout { offer, accept },
        };
        self.outcome_payouts.push(enum_payout);
        self
    }

    pub fn build(self) -> ContractResult<EnumDescriptor> {
        if self.outcome_payouts.is_empty() {
            return Err(ContractError::MissingOutcomePayouts);
        }
        Ok(EnumDescriptor {
            outcome_payouts: self.outcome_payouts,
        })
    }
}

#[derive(Default)]
#[must_use]
/// Builder for `NumericalDescriptor`
pub struct NumericalDescriptorBuilder {
    payout_points: HashMap<u64, Vec<PayoutPoint>>,
    rounding_intervals: Vec<RoundingInterval>,
    difference_params: Option<DifferenceParams>,
    oracle_numeric_info: Option<OracleNumericInfo>,
}

impl NumericalDescriptorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_payout_point(
        mut self,
        piece_number: u64,
        outcome: u64,
        payout: u64,
        precision: u16,
    ) -> Self {
        let payout_point = PayoutPoint {
            event_outcome: outcome,
            outcome_payout: payout,
            extra_precision: precision,
        };

        self.payout_points
            .entry(piece_number)
            .or_default()
            .push(payout_point);
        self
    }

    pub fn add_rounding_interval(mut self, begin: u64, rounding_mod: u64) -> Self {
        self.rounding_intervals.push(RoundingInterval {
            begin_interval: begin,
            rounding_mod,
        });
        self
    }

    pub fn set_difference_params(
        mut self,
        max_error_exp: usize,
        min_support_exp: usize,
        maximize_coverage: bool,
    ) -> Self {
        self.difference_params = Some(DifferenceParams {
            max_error_exp,
            min_support_exp,
            maximize_coverage,
        });
        self
    }

    pub fn set_oracle_numeric_info(mut self, base: usize, nb_digits: Vec<usize>) -> Self {
        self.oracle_numeric_info = Some(OracleNumericInfo { base, nb_digits });
        self
    }

    pub fn build(self) -> ContractResult<NumericalDescriptor> {
        if self.payout_points.len() <= 1 {
            return Err(ContractError::InvalidPayoutPoints);
        }
        if self.rounding_intervals.is_empty() {
            return Err(ContractError::InvalidRoundingInterval);
        }
        if self.oracle_numeric_info.is_none() {
            return Err(ContractError::MissingOracleNumericInfo);
        }

        let payout_function_peices = {
            let mut payout_function_peices = Vec::new();
            for i in 1..=self.payout_points.len()  {
                // handle error and return error
                let payout_points = self
                    .payout_points
                    .get(&(i as u64))
                    .ok_or_else(|| ContractError::InvalidPayoutFunctionPieceSequence)?;
                payout_function_peices.push(PayoutFunctionPiece::PolynomialPayoutCurvePiece(
                    PolynomialPayoutCurvePiece::new(payout_points.clone())?,
                ));
            }
            payout_function_peices
        };

        let payout_function = PayoutFunction::new(payout_function_peices)?;

        Ok(NumericalDescriptor {
            payout_function,
            rounding_intervals: RoundingIntervals {
                intervals: self.rounding_intervals,
            },
            difference_params: self.difference_params,
            oracle_numeric_infos: self
                .oracle_numeric_info
                .ok_or_else(|| ContractError::MissingOracleNumericInfo)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_descriptor_builder_success() {
        let descriptor = EnumDescriptorBuilder::new()
            .add_payout("win".to_string(), 300, 0)
            .add_payout("lose".to_string(), 0, 300)
            .build();

        assert!(descriptor.is_ok());
        let descriptor = descriptor.unwrap();
        assert_eq!(descriptor.outcome_payouts.len(), 2);

        let first_payout = &descriptor.outcome_payouts[0];
        assert_eq!(first_payout.outcome, "win");
        assert_eq!(first_payout.payout.offer, 300);
        assert_eq!(first_payout.payout.accept, 0);

        let second_payout = &descriptor.outcome_payouts[1];
        assert_eq!(second_payout.outcome, "lose");
        assert_eq!(second_payout.payout.offer, 0);
        assert_eq!(second_payout.payout.accept, 300);
    }

    #[test]
    fn test_enum_descriptor_builder_empty_error() {
        let descriptor = EnumDescriptorBuilder::new().build();
        assert!(descriptor.is_err());
        assert!(matches!(
            descriptor.unwrap_err(),
            ContractError::MissingOutcomePayouts
        ));
    }

    #[test]
    fn test_numerical_descriptor_builder_success() {
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(1, 100, 200, 2)
            .add_payout_point(1, 200, 400, 2)
            .add_payout_point(2, 200, 400, 2)
            .add_payout_point(2, 300, 500, 2)
            .add_rounding_interval(0, 10)
            .set_difference_params(5, 3, true)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();

        assert!(descriptor.is_ok());
        let descriptor = descriptor.unwrap();

        assert_eq!(descriptor.rounding_intervals.intervals.len(), 1);
        assert_eq!(descriptor.rounding_intervals.intervals[0].begin_interval, 0);
        assert_eq!(descriptor.rounding_intervals.intervals[0].rounding_mod, 10);

        let diff_params = descriptor.difference_params.unwrap();
        assert_eq!(diff_params.max_error_exp, 5);
        assert_eq!(diff_params.min_support_exp, 3);
        assert!(diff_params.maximize_coverage);

        assert_eq!(descriptor.oracle_numeric_infos.base, 10);
        assert_eq!(descriptor.oracle_numeric_infos.nb_digits, vec![2, 3]);
    }

    #[test]
    fn test_numerical_descriptor_builder_validation_errors() {
        // Test empty payout points
        let descriptor = NumericalDescriptorBuilder::new()
            .add_rounding_interval(0, 10)
            .set_difference_params(5, 3, true)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();
        assert!(descriptor.is_err());
        assert!(matches!(
            descriptor.unwrap_err(),
            ContractError::InvalidPayoutPoints
        ));

        // Test empty rounding intervals
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(1, 100, 200, 2)
            .set_difference_params(5, 3, true)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();
        assert!(descriptor.is_err());
        assert!(matches!(
            descriptor.unwrap_err(),
            ContractError::InvalidRoundingInterval
        ));

        // Test missing oracle numeric info
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(1, 100, 200, 2)
            .add_rounding_interval(0, 10)
            .set_difference_params(5, 3, true)
            .build();
        assert!(descriptor.is_err());
        assert!(matches!(
            descriptor.unwrap_err(),
            ContractError::MissingOracleNumericInfo
        ));
    }

    #[test]
    fn test_numerical_descriptor_multiple_payout_points() {
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(1, 100, 200, 2)
            .add_payout_point(1, 200, 300, 2)
            .add_payout_point(2, 200, 300, 2)
            .add_payout_point(2, 400, 300, 2)
            .add_rounding_interval(0, 10)
            .set_difference_params(5, 3, true)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();

        assert!(descriptor.is_ok());
    }

    #[test]
    fn test_numerical_descriptor_multiple_rounding_intervals() {
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(1, 100, 200, 2)
            .add_payout_point(1, 200, 300, 2)
            .add_payout_point(2, 200, 300, 2)
            .add_payout_point(2, 300, 400, 2)
            .add_rounding_interval(0, 10)
            .add_rounding_interval(10, 20)
            .add_rounding_interval(20, 30)
            .set_difference_params(5, 3, true)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();

        assert!(descriptor.is_ok());
        let descriptor = descriptor.unwrap();
        assert_eq!(descriptor.rounding_intervals.intervals.len(), 3);
    }

    #[test]
    fn test_optional_difference_params() {
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(1, 100, 200, 2)
            .add_payout_point(1, 200, 300, 2)
            .add_payout_point(2, 200, 300, 2)
            .add_payout_point(2, 300, 400, 2)
            .add_rounding_interval(0, 10)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();

        assert!(descriptor.is_ok());
        let descriptor = descriptor.unwrap();
        assert!(descriptor.difference_params.is_none());
    }
}
