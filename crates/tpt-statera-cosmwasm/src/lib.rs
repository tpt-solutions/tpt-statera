// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! CosmWasm adapter for tpt-statera (todo 2B.01, 2B.02, 2B.03; ARCHITECTURE AD-8).
//!
//! The contract's `execute` is the only path that changes balances. The `Transfer` message
//! reads the sender, recipient and treasury balances, calls `validate_transfer`, and writes the
//! three new balances only on `Allow`. On `Deny` it returns an error, so CosmWasm reverts the
//! whole message and nothing is written.
//!
//! ## Trust and scope
//!
//! * **Instantiate sets the fee rate and the initial balances.** This is trusted configuration,
//!   so the instantiate caller is trusted (THREAT_MODEL A3). There is no admin message after
//!   instantiation, so the fee rate and the balances cannot be changed later through this contract.
//!   Funding after instantiation is not supported. That is deliberate, and it is not a complete
//!   token system.
//! * **Serialization is the chain's.** CosmWasm executes messages one at a time per chain, so the
//!   in-process lock in `tpt-statera-runtime` is not needed here (AD-7, AD-8). The claim depends on
//!   the chain's ordering guarantees, which the bypass argument in `docs/BYPASS_ARGUMENT.md` states.
//! * **Amounts are `Uint64` on the wire**, serialized as strings, so JSON clients do not lose
//!   precision above 2^53.
//! * Balances are kept under `bal:<address>` keys. Addresses are validated with `addr_validate`.
//!   The treasury and the sender or recipient must be different accounts.

use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint64,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tpt_statera_core::{validate_transfer, SystemState, TransferProposal, ValidationOutcome};

/// Storage key prefix for balances.
const BAL_PREFIX: &[u8] = b"bal:";
/// Storage key for the configuration.
const CONFIG_KEY: &[u8] = b"cfg";

/// Contract errors. Any error reverts the message.
#[derive(Debug, Error)]
pub enum ContractError {
    /// A standard CosmWasm error (address validation, serialization, storage).
    #[error("{0}")]
    Std(#[from] StdError),
    /// `validate_transfer` returned `Deny`. No state was written.
    #[error("transfer denied by validation")]
    Denied,
    /// Sender and recipient are the same account.
    #[error("sender and recipient must differ")]
    SelfTransfer,
    /// The treasury cannot be a sender or a recipient.
    #[error("the treasury account cannot send or receive transfers")]
    TreasuryParty,
    /// The fee rate exceeds 10 000 bps.
    #[error("fee rate must be at most 10000 bps")]
    InvalidFeeRate,
    /// The initial balances list contains the same address twice.
    #[error("duplicate account in initial balances: {0}")]
    DuplicateAccount(String),
    /// Adding funds would overflow `u64`.
    #[error("balance overflow for {0}")]
    BalanceOverflow(String),
}

/// Instantiate message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstantiateMsg {
    /// Address that collects fees. Must not be a sender or recipient of any transfer.
    pub treasury: String,
    /// Fee rate in basis points. Trusted configuration, at most 10 000.
    pub fee_rate_bps: u64,
    /// Initial balances. Addresses must be unique.
    pub initial_balances: Vec<(String, Uint64)>,
}

/// Execute messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Transfer `amount` from the message sender to `to`. The fee goes to the treasury.
    Transfer {
        /// Recipient address.
        to: String,
        /// Amount the recipient receives, in whole units.
        amount: Uint64,
    },
}

/// Query messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Balance of one address. Zero if the address has no balance entry.
    Balance {
        /// Address to query.
        address: String,
    },
    /// Configuration.
    Config {},
}

/// Response to `Balance`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BalanceResponse {
    /// The balance.
    pub balance: Uint64,
}

/// Response to `Config`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigResponse {
    /// Treasury address.
    pub treasury: String,
    /// Fee rate in basis points.
    pub fee_rate_bps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Config {
    treasury: Addr,
    fee_rate_bps: u64,
}

fn bal_key(addr: &Addr) -> Vec<u8> {
    let mut k = BAL_PREFIX.to_vec();
    k.extend_from_slice(addr.as_bytes());
    k
}

fn load_balance(storage: &dyn Storage, addr: &Addr) -> StdResult<u64> {
    match storage.get(&bal_key(addr)) {
        None => Ok(0),
        Some(bytes) => {
            let arr: [u8; 8] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| StdError::msg("corrupt balance entry"))?;
            Ok(u64::from_be_bytes(arr))
        }
    }
}

fn store_balance(storage: &mut dyn Storage, addr: &Addr, value: u64) {
    storage.set(&bal_key(addr), &value.to_be_bytes());
}

fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    let bytes = storage
        .get(CONFIG_KEY)
        .ok_or_else(|| StdError::msg("config not found"))?;
    cosmwasm_std::from_json(bytes)
}

/// Instantiate the contract. See the module docs for the trust model.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if msg.fee_rate_bps > 10_000 {
        return Err(ContractError::InvalidFeeRate);
    }
    let treasury = deps.api.addr_validate(&msg.treasury)?;
    let config = Config {
        treasury: treasury.clone(),
        fee_rate_bps: msg.fee_rate_bps,
    };
    deps.storage
        .set(CONFIG_KEY, &cosmwasm_std::to_json_vec(&config)?);

    let mut seen = std::collections::BTreeSet::new();
    for (addr, amount) in &msg.initial_balances {
        let addr = deps.api.addr_validate(addr)?;
        if !seen.insert(addr.to_string()) {
            return Err(ContractError::DuplicateAccount(addr.to_string()));
        }
        store_balance(deps.storage, &addr, amount.u64());
    }
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("treasury", treasury)
        .add_attribute("fee_rate_bps", msg.fee_rate_bps.to_string()))
}

/// Execute a message. `Transfer` is the only state-changing path.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { to, amount } => transfer(deps, info.sender, to, amount.u64()),
    }
}

fn transfer(
    deps: DepsMut,
    sender: Addr,
    to: String,
    amount: u64,
) -> Result<Response, ContractError> {
    let config = load_config(deps.storage)?;
    let recipient = deps.api.addr_validate(&to)?;
    if sender == recipient {
        return Err(ContractError::SelfTransfer);
    }
    if sender == config.treasury || recipient == config.treasury {
        return Err(ContractError::TreasuryParty);
    }

    let s = load_balance(deps.storage, &sender)?;
    let r = load_balance(deps.storage, &recipient)?;
    let t = load_balance(deps.storage, &config.treasury)?;

    let outcome = validate_transfer(
        SystemState {
            sender_balance: s,
            recipient_balance: r,
            treasury_balance: t,
        },
        TransferProposal {
            amount,
            fee_rate_bps: config.fee_rate_bps,
        },
    );
    match outcome {
        ValidationOutcome::Allow {
            new_sender_balance,
            new_recipient_balance,
            new_treasury_balance,
            rounded_fee,
            dust_retained_by_sender,
        } => {
            store_balance(deps.storage, &sender, new_sender_balance);
            store_balance(deps.storage, &recipient, new_recipient_balance);
            store_balance(deps.storage, &config.treasury, new_treasury_balance);
            Ok(Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("from", sender)
                .add_attribute("to", recipient)
                .add_attribute("amount", amount.to_string())
                .add_attribute("fee", rounded_fee.to_string())
                .add_attribute(
                    "dust_retained_by_sender",
                    dust_retained_by_sender.to_string(),
                ))
        }
        // Returning an error reverts the message, so nothing above is persisted.
        ValidationOutcome::Deny => Err(ContractError::Denied),
    }
}

/// Query balances and configuration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => {
            let addr = deps.api.addr_validate(&address)?;
            let balance = load_balance(deps.storage, &addr)?;
            to_json_binary(&BalanceResponse {
                balance: Uint64::new(balance),
            })
        }
        QueryMsg::Config {} => {
            let c = load_config(deps.storage)?;
            to_json_binary(&ConfigResponse {
                treasury: c.treasury.into_string(),
                fee_rate_bps: c.fee_rate_bps,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_json;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::Storage;

    /// Sum of the balances of every test account. Test-only. Storage iteration is not used, because
    /// the on-chain build leaves the iterator feature off. The test accounts are listed explicitly.
    fn total_supply(storage: &dyn Storage) -> u128 {
        ["alice", "bob", "treasury"]
            .iter()
            .map(|n| load_balance(storage, &Addr::unchecked(a(n))).unwrap() as u128)
            .sum()
    }

    /// Valid bech32 test address for a short name.
    fn a(name: &str) -> String {
        MockApi::default().addr_make(name).to_string()
    }

    fn init(
        fee_bps: u64,
    ) -> cosmwasm_std::OwnedDeps<
        cosmwasm_std::testing::MockStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    > {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            treasury: a("treasury"),
            fee_rate_bps: fee_bps,
            initial_balances: vec![
                (a("alice"), Uint64::new(1_000_000)),
                (a("bob"), Uint64::new(0)),
            ],
        };
        instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&Addr::unchecked(a("admin")), &[]),
            msg,
        )
        .unwrap();
        deps
    }

    fn balance(deps: cosmwasm_std::Deps, who: &str) -> u64 {
        let bin = query(deps, mock_env(), QueryMsg::Balance { address: a(who) }).unwrap();
        from_json::<BalanceResponse>(&bin).unwrap().balance.u64()
    }

    fn transfer_msg(
        deps: cosmwasm_std::DepsMut,
        from: &str,
        to: &str,
        amount: u64,
    ) -> Result<Response, ContractError> {
        execute(
            deps,
            mock_env(),
            message_info(&Addr::unchecked(a(from)), &[]),
            ExecuteMsg::Transfer {
                to: a(to),
                amount: Uint64::new(amount),
            },
        )
    }

    #[test]
    fn transfer_moves_amount_and_fee_and_conserves_supply() {
        let mut deps = init(100); // 1%
        let before = total_supply(&deps.storage);
        let resp = transfer_msg(deps.as_mut(), "alice", "bob", 500).unwrap();
        assert!(resp
            .attributes
            .iter()
            .any(|a| a.key == "fee" && a.value == "5"));
        assert_eq!(balance(deps.as_ref(), "alice"), 999_495);
        assert_eq!(balance(deps.as_ref(), "bob"), 500);
        assert_eq!(balance(deps.as_ref(), "treasury"), 5);
        assert_eq!(total_supply(&deps.storage), before);
    }

    #[test]
    fn denied_transfer_errors_and_writes_nothing() {
        let mut deps = init(0);
        let err = transfer_msg(deps.as_mut(), "bob", "alice", 1).unwrap_err();
        assert!(matches!(err, ContractError::Denied));
        assert_eq!(balance(deps.as_ref(), "bob"), 0);
        assert_eq!(balance(deps.as_ref(), "alice"), 1_000_000);
    }

    #[test]
    fn self_transfer_and_treasury_party_are_rejected() {
        let mut deps = init(0);
        assert!(matches!(
            transfer_msg(deps.as_mut(), "alice", "alice", 1).unwrap_err(),
            ContractError::SelfTransfer
        ));
        assert!(matches!(
            transfer_msg(deps.as_mut(), "alice", "treasury", 1).unwrap_err(),
            ContractError::TreasuryParty
        ));
        assert!(matches!(
            transfer_msg(deps.as_mut(), "treasury", "bob", 1).unwrap_err(),
            ContractError::TreasuryParty
        ));
    }

    #[test]
    fn instantiate_rejects_bad_fee_rate_and_duplicates() {
        let mut deps = mock_dependencies();
        let bad = InstantiateMsg {
            treasury: a("treasury"),
            fee_rate_bps: 10_001,
            initial_balances: vec![],
        };
        assert!(matches!(
            instantiate(
                deps.as_mut(),
                mock_env(),
                message_info(&Addr::unchecked(a("admin")), &[]),
                bad
            )
            .unwrap_err(),
            ContractError::InvalidFeeRate
        ));
        let dup = InstantiateMsg {
            treasury: a("treasury"),
            fee_rate_bps: 0,
            initial_balances: vec![(a("alice"), Uint64::new(1)), (a("alice"), Uint64::new(2))],
        };
        assert!(matches!(
            instantiate(
                deps.as_mut(),
                mock_env(),
                message_info(&Addr::unchecked(a("admin")), &[]),
                dup
            )
            .unwrap_err(),
            ContractError::DuplicateAccount(_)
        ));
    }

    #[test]
    fn config_query_returns_treasury_and_rate() {
        let deps = init(15);
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let c: ConfigResponse = from_json(&bin).unwrap();
        assert_eq!(
            c,
            ConfigResponse {
                treasury: a("treasury"),
                fee_rate_bps: 15
            }
        );
    }
}
