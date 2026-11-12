// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
//! Simulator tests for the CosmWasm adapter, using `cw-multi-test` (pinned 3.0.1).
//!
//! These check, in a multi-contract simulator, the behaviour that `docs/BYPASS_ARGUMENT.md` cites from
//! the CosmWasm docs: a failing message reverts its own writes, and a failing message in a transaction
//! reverts the whole transaction. The simulator is not a live chain. It is an independent check of the
//! adapter's error path, not of the chain's ordering guarantees.

use cosmwasm_std::{Addr, Coin, CosmosMsg, Uint64, WasmMsg};
use cw_multi_test::{App, ContractWrapper, Executor};
use tpt_statera_cosmwasm::{
    execute, instantiate, query, BalanceResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};

struct Setup {
    app: App,
    contract: Addr,
    alice: Addr,
    bob: Addr,
    treasury: Addr,
}

fn setup(fee_bps: u64) -> Setup {
    let mut app = App::default();
    let alice = app.api().addr_make("alice");
    let bob = app.api().addr_make("bob");
    let treasury = app.api().addr_make("treasury");
    let admin = app.api().addr_make("admin");

    let code = ContractWrapper::new(execute, instantiate, query);
    let code_id = app.store_code(Box::new(code));
    let contract = app
        .instantiate_contract(
            code_id,
            admin,
            &InstantiateMsg {
                treasury: treasury.to_string(),
                fee_rate_bps: fee_bps,
                initial_balances: vec![
                    (alice.to_string(), Uint64::new(1_000_000)),
                    (bob.to_string(), Uint64::new(0)),
                ],
            },
            &[] as &[Coin],
            "tpt-statera",
            None,
        )
        .expect("instantiate");
    Setup {
        app,
        contract,
        alice,
        bob,
        treasury,
    }
}

fn balance(app: &App, contract: &Addr, who: &Addr) -> u64 {
    let r: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            contract.clone(),
            &QueryMsg::Balance {
                address: who.to_string(),
            },
        )
        .expect("query");
    r.balance.u64()
}

fn transfer(to: &Addr, amount: u64) -> ExecuteMsg {
    ExecuteMsg::Transfer {
        to: to.to_string(),
        amount: Uint64::new(amount),
    }
}

#[test]
fn committed_transfer_is_visible_to_queries() {
    let mut s = setup(100); // 1%
    s.app
        .execute_contract(
            s.alice.clone(),
            s.contract.clone(),
            &transfer(&s.bob, 500),
            &[],
        )
        .expect("transfer");
    assert_eq!(balance(&s.app, &s.contract, &s.alice), 999_495);
    assert_eq!(balance(&s.app, &s.contract, &s.bob), 500);
    assert_eq!(balance(&s.app, &s.contract, &s.treasury), 5);
}

#[test]
fn denied_transfer_fails_and_writes_nothing() {
    let mut s = setup(0);
    // Bob has nothing, so this is denied by validation.
    let err = s.app.execute_contract(
        s.bob.clone(),
        s.contract.clone(),
        &transfer(&s.alice, 1),
        &[],
    );
    assert!(err.is_err(), "denied transfer must error");
    assert_eq!(balance(&s.app, &s.contract, &s.bob), 0);
    assert_eq!(balance(&s.app, &s.contract, &s.alice), 1_000_000);
}

#[test]
fn failing_message_reverts_earlier_messages_in_the_same_transaction() {
    // One transaction from alice, two messages. The first is a valid transfer of 400 to bob. The second
    // asks for 2 000 000, which alice cannot cover after the first, so it is denied. Atomicity means the
    // first message is rolled back as well. The first message alone would succeed, so this is a real test.
    let mut s = setup(0);
    let msg = |to: &Addr, amount: u64| -> CosmosMsg {
        WasmMsg::Execute {
            contract_addr: s.contract.to_string(),
            msg: cosmwasm_std::to_json_binary(&transfer(to, amount)).unwrap(),
            funds: vec![],
        }
        .into()
    };
    let msgs = vec![msg(&s.bob, 400), msg(&s.bob, 2_000_000)];
    let err = s.app.execute_multi(s.alice.clone(), msgs);
    assert!(err.is_err(), "transaction with a denied message must fail");
    assert_eq!(
        balance(&s.app, &s.contract, &s.alice),
        1_000_000,
        "first message not rolled back"
    );
    assert_eq!(balance(&s.app, &s.contract, &s.bob), 0);
    assert_eq!(balance(&s.app, &s.contract, &s.treasury), 0);

    // Control: the first message on its own succeeds, so the rollback above is not a side effect of it failing.
    s.app
        .execute_contract(
            s.alice.clone(),
            s.contract.clone(),
            &transfer(&s.bob, 400),
            &[],
        )
        .expect("first message alone succeeds");
    assert_eq!(balance(&s.app, &s.contract, &s.bob), 400);
}

#[test]
fn messages_in_sequence_see_each_others_writes() {
    // Serial execution: the second transfer sees the balance left by the first.
    let mut s = setup(0);
    s.app
        .execute_contract(
            s.alice.clone(),
            s.contract.clone(),
            &transfer(&s.bob, 600_000),
            &[],
        )
        .expect("first");
    s.app
        .execute_contract(
            s.bob.clone(),
            s.contract.clone(),
            &transfer(&s.alice, 600_000),
            &[],
        )
        .expect("second sees first");
    assert_eq!(
        balance(&s.app, &s.contract, &s.alice),
        1_000_000 - 600_000 + 600_000
    );
    assert_eq!(balance(&s.app, &s.contract, &s.bob), 0);
}
