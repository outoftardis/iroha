//! This module contains `Domain` structure and related implementations and trait implementations.
use crate::prelude::*;
use std::collections::HashMap;

type Name = String;

/// Named group of `Account` and `Asset` entities.
#[derive(Debug, Clone)]
pub struct Domain {
    /// Domain name, for example company name.
    pub name: Name,
    /// Accounts of the domain.
    pub accounts: HashMap<<Account as Identifiable>::Id, Account>,
    /// Assets of the domain.
    pub assets: HashMap<<Asset as Identifiable>::Id, Asset>,
}

impl Domain {
    /// Creates new detached `Domain`.
    ///
    /// Should be used for creation of a new `Domain` or while making queries.
    pub fn new(name: Name) -> Self {
        Domain {
            name,
            accounts: HashMap::new(),
            assets: HashMap::new(),
        }
    }
}

impl Identifiable for Domain {
    type Id = Name;
}

/// Iroha Special Instructions module provides `DomainInstruction` enum with all legal types of
/// Domain related instructions as variants, implementations of generic Iroha Special Instructions
/// and the `From/Into` implementations to convert `DomainInstruction` variants into generic ISI.
pub mod isi {
    use super::*;
    use crate::isi::Register;
    use iroha_derive::*;
    use parity_scale_codec::{Decode, Encode};

    /// Enumeration of all legal Domain related Instructions.
    #[derive(Clone, Debug, Io, Encode, Decode)]
    pub enum DomainInstruction {
        /// Variant of the generic `Register` instruction for `Account` --> `Domain`.
        RegisterAccount(Name, Account),
        /// Variant of the generic `Register` instruction for `Asset` --> `Domain`.
        RegisterAsset(Name, Asset),
    }

    impl DomainInstruction {
        /// Executes `DomainInstruction` on the given `WorldStateView`.
        /// Returns `Ok(())` if execution succeeded and `Err(String)` with error message if not.
        pub fn execute(&self, world_state_view: &mut WorldStateView) -> Result<(), String> {
            match self {
                DomainInstruction::RegisterAccount(domain_name, account) => {
                    Register::new(account.clone(), domain_name.clone()).execute(world_state_view)
                }
                DomainInstruction::RegisterAsset(domain_name, asset) => {
                    Register::new(asset.clone(), domain_name.clone()).execute(world_state_view)
                }
            }
        }
    }

    impl From<Register<Domain, Account>> for Instruction {
        fn from(instruction: Register<Domain, Account>) -> Self {
            Instruction::Domain(DomainInstruction::RegisterAccount(
                instruction.destination_id,
                instruction.object,
            ))
        }
    }

    impl Register<Domain, Account> {
        fn execute(&self, world_state_view: &mut WorldStateView) -> Result<(), String> {
            let account = self.object.clone();
            let domain = world_state_view
                .domain(&self.destination_id)
                .ok_or("Failed to find domain.")?;
            if domain.accounts.contains_key(&account.id) {
                Err(format!(
                    "Domain already contains an account with an Id: {:?}",
                    &account.id
                ))
            } else {
                domain.accounts.insert(account.id.clone(), account);
                Ok(())
            }
        }
    }

    impl From<Register<Domain, Asset>> for Instruction {
        fn from(instruction: Register<Domain, Asset>) -> Self {
            Instruction::Domain(DomainInstruction::RegisterAsset(
                instruction.destination_id,
                instruction.object,
            ))
        }
    }

    impl Register<Domain, Asset> {
        fn execute(&self, world_state_view: &mut WorldStateView) -> Result<(), String> {
            let asset = self.object.clone();
            world_state_view
                .domain(&self.destination_id)
                .ok_or("Failed to find domain.")?
                .accounts
                .get_mut(&asset.id.account_id())
                .expect("Failed to find account.")
                .assets
                .insert(asset.id.clone(), asset);
            Ok(())
        }
    }
}
