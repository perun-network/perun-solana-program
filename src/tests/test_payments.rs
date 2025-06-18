// Copyright 2025 - See NOTICE file for copyright holders.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#![cfg(test)]
use crate::tests::setup::setup;

const A: bool = false;
const B: bool = true;

#[tokio::test]
pub async fn test_honest_payment_cross_sameasset() {
    let one_withdrawer = false;
    let mixed_asset = false;

    let bal_contract_after_afund = vec![100, 150];
    let bal_contract_after_bfund = vec![300, 400];
    let bal_contract_after_awdraw = vec![200, 200];
    let bal_contract_after_bwdraw = vec![0, 0];
    let bal_contract_after_final = vec![300, 400];
    let bal_a_after_afund = vec![0, 0];
    let bal_a_after_awdraw = vec![100, 200];
    let bal_b_after_bfund = vec![0, 0];
    let bal_b_after_bwdraw = vec![200, 200];
    let to_send_a = vec![0, 50];

    // Setup the test environment
    let mut bal_a = vec![];
    bal_a.push(100);
    bal_a.push(150);

    let mut bal_b = vec![];
    bal_b.push(200);
    bal_b.push(250);

    let mut t = setup(10, bal_a, bal_b, mixed_asset)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice and Bob fund the channel.
    t.fund(A, mixed_asset).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    t.fund(B, mixed_asset).await;
    t.verify_bal_contract(bal_contract_after_bfund).await;
    t.verify_bal_b(bal_b_after_bfund).await;

    // Update channel off-chain.
    t.send_to_a(to_send_a);

    // Finalize the channel.
    t.finalize();

    // Sign the final state off-chain.
    let sig_a_cc = t.sigs_cc_abi_a();
    let sig_b_cc = t.sigs_cc_abi_b();

    // Call the close instruction on-chain.
    t.close(t.state.clone(), sig_a_cc.clone(), sig_b_cc.clone())
        .await;
    t.verify_state(&t.state).await;
    t.verify_bal_contract(bal_contract_after_final).await;

    // Alice withdraws her funds.
    t.withdraw(A, one_withdrawer, mixed_asset).await;
    t.verify_bal_a(bal_a_after_awdraw).await;
    t.verify_bal_contract(bal_contract_after_awdraw).await;

    // Bob withdraws his funds.
    t.withdraw(B, one_withdrawer, mixed_asset).await;
    t.verify_bal_b(bal_b_after_bwdraw).await;
    t.verify_bal_contract(bal_contract_after_bwdraw).await;
}

#[tokio::test]
pub async fn test_honest_payment_cross_mixedasset() {
    let one_withdrawer = false;
    let mixed_asset = true;

    let bal_contract_after_afund = vec![100, 150];
    let bal_contract_after_bfund = vec![300, 400];
    let bal_contract_after_awdraw = vec![200, 200];
    let bal_contract_after_bwdraw = vec![0, 0];
    let bal_contract_after_final = vec![300, 400];
    let bal_a_after_afund = vec![0, 0];
    let bal_a_after_awdraw = vec![100, 200];
    let bal_b_after_bfund = vec![0, 0];
    let bal_b_after_bwdraw = vec![200, 200];
    let to_send_a = vec![0, 50];

    // Setup the test environment
    let mut bal_a = vec![];
    bal_a.push(100);
    bal_a.push(150);

    let mut bal_b = vec![];
    bal_b.push(200);
    bal_b.push(250);

    let mut t = setup(10, bal_a, bal_b, mixed_asset)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice and Bob fund the channel.
    t.fund(A, mixed_asset).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    t.fund(B, mixed_asset).await;
    t.verify_bal_contract(bal_contract_after_bfund).await;
    t.verify_bal_b(bal_b_after_bfund).await;

    // Update channel off-chain.
    t.send_to_a(to_send_a);

    // Finalize the channel.
    t.finalize();

    // Sign the final state off-chain.
    let sig_a_cc = t.sigs_cc_abi_a();
    let sig_b_cc = t.sigs_cc_abi_b();

    // Call the close instruction on-chain.
    t.close(t.state.clone(), sig_a_cc.clone(), sig_b_cc.clone())
        .await;
    t.verify_state(&t.state).await;
    t.verify_bal_contract(bal_contract_after_final).await;

    // Alice withdraws her funds.
    t.withdraw(A, one_withdrawer, mixed_asset).await;
    t.verify_bal_a(bal_a_after_awdraw).await;
    t.verify_bal_contract(bal_contract_after_awdraw).await;

    // Bob withdraws his funds.
    t.withdraw(B, one_withdrawer, mixed_asset).await;
    t.verify_bal_b(bal_b_after_bwdraw).await;
    t.verify_bal_contract(bal_contract_after_bwdraw).await;
}
