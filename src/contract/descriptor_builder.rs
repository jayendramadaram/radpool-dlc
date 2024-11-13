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
pub struct EnumDescriptorBuilder {
    outcome_payouts: Vec<EnumerationPayout>,
}

impl EnumDescriptorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_payout(mut self, outcome: String, offer: u64, accept: u64) -> Self {
        let enum_payout = EnumerationPayout {
            outcome,
            payout: Payout { accept, offer },
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
pub struct NumericalDescriptorBuilder {
    payout_points: Vec<PayoutPoint>,
    rounding_intervals: Vec<RoundingInterval>,
    difference_params: Option<DifferenceParams>,
    oracle_numeric_info: Option<OracleNumericInfo>,
}

impl NumericalDescriptorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_payout_point(mut self, outcome: u64, payout: u64, precision: u16) -> Self {
        self.payout_points.push(PayoutPoint {
            event_outcome: outcome,
            outcome_payout: payout,
            extra_precision: precision,
        });
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
        if self.payout_points.len() <= 0 {
            return Err(ContractError::InvalidPayoutPoints("at most more than one payout point is required".to_string()));
        }
        if self.rounding_intervals.is_empty() {
            return Err(ContractError::InvalidRoundingInterval("at least one rounding interval is required".to_string()));
        }
        if self.oracle_numeric_info.is_none() {
            return Err(ContractError::MissingOracleNumericInfo);
        }

        let payout_function =
            PayoutFunction::new(vec![PayoutFunctionPiece::PolynomialPayoutCurvePiece(
                PolynomialPayoutCurvePiece::new(self.payout_points).unwrap(),
            )])
            .unwrap();

        Ok(NumericalDescriptor {
            payout_function,
            rounding_intervals: RoundingIntervals {
                intervals: self.rounding_intervals,
            },
            difference_params: self.difference_params,
            oracle_numeric_infos: self.oracle_numeric_info.unwrap(),
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
        assert_eq!(descriptor.unwrap_err(), ContractError::MissingOutcomePayouts);
    }

    #[test]
    fn test_numerical_descriptor_builder_success() {
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(100, 200, 2)
            .add_payout_point(200, 300, 2)
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
        assert_eq!(
            descriptor.unwrap_err(),
            ContractError::InvalidPayoutPoints("at most more than one payout point is required".to_string())
        );

        // Test empty rounding intervals
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(100, 200, 2)
            .set_difference_params(5, 3, true)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();
        assert!(descriptor.is_err());
        assert_eq!(
            descriptor.unwrap_err(),
            ContractError::InvalidRoundingInterval("at least one rounding interval is required".to_string())
        );

        // Test missing oracle numeric info
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(100, 200, 2)
            .add_rounding_interval(0, 10)
            .set_difference_params(5, 3, true)
            .build();
        assert!(descriptor.is_err());
        assert_eq!(descriptor.unwrap_err(), ContractError::MissingOracleNumericInfo);
    }

    #[test]
    fn test_numerical_descriptor_multiple_payout_points() {
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(100, 200, 2)
            .add_payout_point(200, 300, 2)
            .add_payout_point(300, 400, 2)
            .add_rounding_interval(0, 10)
            .set_difference_params(5, 3, true)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();

        assert!(descriptor.is_ok());
    }

    #[test]
    fn test_numerical_descriptor_multiple_rounding_intervals() {
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(100, 200, 2)
            .add_payout_point(200, 300, 2)
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
            .add_payout_point(100, 200, 2)
            .add_payout_point(200, 300, 2)
            .add_rounding_interval(0, 10)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();

        assert!(descriptor.is_ok());
        let descriptor = descriptor.unwrap();
        assert!(descriptor.difference_params.is_none());
    }
}
