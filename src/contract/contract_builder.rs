use dlc::secp256k1_zkp::XOnlyPublicKey;
use dlc_manager::contract::{
    contract_input::{ContractInput, ContractInputInfo, OracleInput},
    ContractDescriptor,
};

use super::errors::{ContractError, ContractResult};

/// Builder for `ContractInput`
pub struct ContractBuilder {
    contract: ContractInput,
}


impl ContractBuilder {
    /// Returns a new `ContractBuilder` with default values.
    pub fn new() -> Self {
        ContractBuilder {
            contract: ContractInput {
                fee_rate: 0,
                accept_collateral: 0,
                offer_collateral: 0,
                contract_infos: Vec::new(),
            },
        }
    }

    pub fn fee_rate(mut self, fee_rate: u64) -> Self {
        self.contract.fee_rate = fee_rate;
        self
    }

    pub fn accept_collateral(mut self, accept_collateral: u64) -> Self {
        self.contract.accept_collateral = accept_collateral;
        self
    }

    pub fn offer_collateral(mut self, offer_collateral: u64) -> Self {
        self.contract.offer_collateral = offer_collateral;
        self
    }

    pub fn with_contract_info(mut self, contract_info: ContractInputInfo) -> Self {
        self.contract.contract_infos.push(contract_info);
        self
    }

    pub fn create_contract_info(
        descriptor: ContractDescriptor,
        public_keys: Vec<XOnlyPublicKey>,
        event_id: String,
        threshold: u16,
    ) -> ContractResult<ContractInputInfo> {
        if public_keys.is_empty() {
            return Err(ContractError::MissingOracles);
        }  

        // pubkeys would always be in range of u16
        if threshold > public_keys.len() as u16 || threshold == 0 {
            return Err(ContractError::InvalidThreshold);
        }

        Ok(ContractInputInfo {
            contract_descriptor: descriptor,
            oracles: OracleInput {
                public_keys,
                event_id,
                threshold,
            },
        })
    }

    pub fn build(self) -> ContractResult<ContractInput> {
        if self.contract.contract_infos.is_empty() {
            return Err(ContractError::MissingContractInfo);
        }
        Ok(self.contract)
    }
}

#[cfg(test)]
mod tests {
    use crate::contract::descriptor_builder::NumericalDescriptorBuilder;

    use super::*;

    use lightning::bitcoin::secp256k1::{Secp256k1, SecretKey};

    fn create_test_xonly_pubkey() -> XOnlyPublicKey {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[1; 32]).unwrap();
        let (pubkey, _parity) = secret_key.x_only_public_key(&secp);
        pubkey
    }


    fn create_test_numerical_descriptor() -> ContractDescriptor {
        let descriptor = NumericalDescriptorBuilder::new()
            .add_payout_point(1, 100, 200, 2)
            .add_payout_point(1, 200, 300, 2)
            .add_payout_point(2, 200, 300, 2)
            .add_payout_point(2, 300, 400, 2)
            .add_rounding_interval(0, 10)
            .set_difference_params(5, 3, true)
            .set_oracle_numeric_info(10, vec![2, 3])
            .build();

        ContractDescriptor::Numerical(descriptor.unwrap())
    }

    #[test]
    fn test_contract_builder_success() {
        let pubkey = create_test_xonly_pubkey();
        let contract_info = ContractBuilder::create_contract_info(
            create_test_numerical_descriptor(),
            vec![pubkey],
            "btcusd1731397577".to_string(),
            1,
        );

        assert!(contract_info.is_ok());
        let contract_info = contract_info.unwrap();

        let contract = ContractBuilder::new()
            .fee_rate(1000)
            .offer_collateral(5000)
            .accept_collateral(5000)
            .with_contract_info(contract_info)
            .build();

        assert!(contract.is_ok());
        let contract = contract.unwrap();
        assert_eq!(contract.fee_rate, 1000);
        assert_eq!(contract.offer_collateral, 5000);
        assert_eq!(contract.accept_collateral, 5000);
        assert_eq!(contract.contract_infos.len(), 1);
    }

    #[test]
    fn test_contract_builder_multiple_infos() {
        let pubkey = create_test_xonly_pubkey();
        let contract_info1 = ContractBuilder::create_contract_info(
            create_test_numerical_descriptor(),
            vec![pubkey],
            "btcusd1731397577".to_string(),
            1,
        );
        let contract_info2 = ContractBuilder::create_contract_info(
            create_test_numerical_descriptor(),
            vec![pubkey],
            "btcusd1731397577".to_string(),
            1,
        );

        assert!(contract_info1.is_ok());
        let contract_info1 = contract_info1.unwrap();
        assert!(contract_info2.is_ok());
        let contract_info2 = contract_info2.unwrap();

        let contract = ContractBuilder::new()
            .fee_rate(1000)
            .offer_collateral(5000)
            .accept_collateral(5000)
            .with_contract_info(contract_info1)
            .with_contract_info(contract_info2)
            .build();

        assert!(contract.is_ok());
        assert_eq!(contract.unwrap().contract_infos.len(), 2);
    }

    #[test]
    fn test_contract_builder_validation_errors() {
        // Test missing contract info
        let contract = ContractBuilder::new()
            .fee_rate(1000)
            .offer_collateral(5000)
            .accept_collateral(5000)
            .build();
        assert!(contract.is_err());
        matches!(contract.unwrap_err(), ContractError::MissingContractInfo);
    }

    #[test]
    fn test_contract_builder_missing_oracles() {
        let contract_info = ContractBuilder::create_contract_info(
            create_test_numerical_descriptor(),
            vec![],
            "btcusd1731397577".to_string(),
            1,
        );
        assert!(contract_info.is_err());
        assert!(matches!(
            contract_info.unwrap_err(),
            ContractError::MissingOracles
        ));
    }

    #[test]
    fn test_contract_builder_invalid_threshold() {
        let contract_info = ContractBuilder::create_contract_info(
            create_test_numerical_descriptor(),
            vec![create_test_xonly_pubkey()],
            "btcusd1731397577".to_string(),
            0,
        );
        assert!(contract_info.is_err());
        assert!(matches!(
            contract_info.unwrap_err(),
            ContractError::InvalidThreshold
        ));
    }
}
