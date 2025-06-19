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
pub async fn test_honest_payment_cross_same_assets() {
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
    let bal_a = vec![100, 150];
    let bal_b = vec![200, 250];

    let mut t = setup(10, bal_a, bal_b, mixed_asset, one_withdrawer)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice and Bob fund the channel.
    t.fund(A).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    t.fund(B).await;
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
    t.withdraw(A).await;
    t.verify_bal_a(bal_a_after_awdraw).await;
    t.verify_bal_contract(bal_contract_after_awdraw).await;

    // Bob withdraws his funds.
    t.withdraw(B).await;
    t.verify_bal_b(bal_b_after_bwdraw).await;
    t.verify_bal_contract(bal_contract_after_bwdraw).await;
}

#[tokio::test]
pub async fn test_honest_payment_cross_mixed_asset() {
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
    let bal_a = vec![100, 150];
    let bal_b = vec![200, 250];

    let mut t = setup(10, bal_a, bal_b, mixed_asset, one_withdrawer)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice and Bob fund the channel.
    t.fund(A).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    t.fund(B).await;
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
    t.withdraw(A).await;
    t.verify_bal_a(bal_a_after_awdraw).await;
    t.verify_bal_contract(bal_contract_after_awdraw).await;

    // Bob withdraws his funds.
    t.withdraw(B).await;
    t.verify_bal_b(bal_b_after_bwdraw).await;
    t.verify_bal_contract(bal_contract_after_bwdraw).await;
}

#[tokio::test]
pub async fn test_funding_abort_cross_mixed_assets() {
    let one_withdrawer = false;
    let mixed_asset = true;

    let bal_contract_after_afund = vec![100, 150];
    let bal_contract_after_abort = vec![0, 0];

    let bal_a_after_afund = vec![0, 0];
    let bal_a_after_abort = vec![100, 150];

    // Setup the test environment
    let bal_a = vec![100, 150];
    let bal_b = vec![200, 250];

    let mut t = setup(10, bal_a, bal_b, mixed_asset, one_withdrawer)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice funds the channel.
    t.fund(A).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    // Alice aborts funding.
    t.abort(A).await;
    t.verify_bal_contract(bal_contract_after_abort).await;
    t.verify_bal_a(bal_a_after_abort).await;
}

#[tokio::test]
pub async fn test_funding_abort_cross_same_assets() {
    let one_withdrawer = false;
    let mixed_asset = false;

    let bal_contract_after_afund = vec![100, 150];
    let bal_contract_after_abort = vec![0, 0];

    let bal_a_after_afund = vec![0, 0];
    let bal_a_after_abort = vec![100, 150];

    // Setup the test environment
    let bal_a = vec![100, 150];
    let bal_b = vec![200, 250];

    let mut t = setup(10, bal_a, bal_b, mixed_asset, one_withdrawer)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice funds the channel.
    t.fund(A).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    // Alice aborts funding.
    t.abort(A).await;
    t.verify_bal_contract(bal_contract_after_abort).await;
    t.verify_bal_a(bal_a_after_abort).await;
}

#[tokio::test]
pub async fn test_dispute_cross_same_assets() {
    let one_withdrawer = false;
    let mixed_asset = false;

    let bal_contract_after_afund = vec![100, 150];
    let bal_contract_after_bfund = vec![300, 400];

    let bal_contract_after_fclose = vec![300, 400];
    let bal_contract_after_awdraw = vec![200, 200];
    let bal_contract_after_bwdraw = vec![0, 0];

    let bal_a_after_afund = vec![0, 0];
    let bal_a_after_awdraw = vec![100, 200];

    let bal_b_after_bfund = vec![0, 0];
    let bal_b_after_bwdraw = vec![200, 200];

    let to_send_a = vec![0, 50];

    // Setup the test environment
    let bal_a = vec![100, 150];
    let bal_b = vec![200, 250];
    let mut t = setup(10, bal_a, bal_b, mixed_asset, one_withdrawer)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice and Bob fund the channel.
    t.fund(A).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    t.fund(B).await;
    t.verify_bal_contract(bal_contract_after_bfund).await;
    t.verify_bal_b(bal_b_after_bfund).await;

    // Update channel off-chain.
    t.send_to_a(to_send_a);

    // A disputes the channel.
    let sig_a_cc = t.sigs_cc_abi_a();
    let sig_b_cc = t.sigs_cc_abi_b();
    t.dispute(A, t.state.clone(), sig_a_cc.clone(), sig_b_cc.clone())
        .await;
    t.verify_state(&t.state).await;

    t.advance_clock_by(15_000).await; // Forward the clock to allow for dispute resolution.

    // A force-closes the channel.
    t.force_close(A).await;
    t.verify_state(&t.state).await;
    t.verify_bal_contract(bal_contract_after_fclose).await;

    // A withdraws her funds.
    t.withdraw(A).await;
    t.verify_bal_a(bal_a_after_awdraw).await;
    t.verify_bal_contract(bal_contract_after_awdraw).await;

    // B withdraws his funds.
    t.withdraw(B).await;
    t.verify_bal_b(bal_b_after_bwdraw).await;
    t.verify_bal_contract(bal_contract_after_bwdraw).await;
}

#[tokio::test]
pub async fn test_dispute_cross_mixed_assets() {
    let one_withdrawer = false;
    let mixed_asset = true;

    let bal_contract_after_afund = vec![100, 150];
    let bal_contract_after_bfund = vec![300, 400];

    let bal_contract_after_fclose = vec![300, 400];
    let bal_contract_after_awdraw = vec![200, 200];
    let bal_contract_after_bwdraw = vec![0, 0];

    let bal_a_after_afund = vec![0, 0];
    let bal_a_after_awdraw = vec![100, 200];

    let bal_b_after_bfund = vec![0, 0];
    let bal_b_after_bwdraw = vec![200, 200];

    let to_send_a = vec![0, 50];

    // Setup the test environment
    let bal_a = vec![100, 150];
    let bal_b = vec![200, 250];
    let mut t = setup(10, bal_a, bal_b, mixed_asset, one_withdrawer)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice and Bob fund the channel.
    t.fund(A).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    t.fund(B).await;
    t.verify_bal_contract(bal_contract_after_bfund).await;
    t.verify_bal_b(bal_b_after_bfund).await;

    // Update channel off-chain.
    t.send_to_a(to_send_a);

    // A disputes the channel.
    let sig_a_cc = t.sigs_cc_abi_a();
    let sig_b_cc = t.sigs_cc_abi_b();
    t.dispute(A, t.state.clone(), sig_a_cc.clone(), sig_b_cc.clone())
        .await;
    t.verify_state(&t.state).await;

    t.advance_clock_by(15_000).await; // Forward the clock to allow for dispute resolution.

    // A force-closes the channel.
    t.force_close(A).await;
    t.verify_state(&t.state).await;
    t.verify_bal_contract(bal_contract_after_fclose).await;

    // A withdraws her funds.
    t.withdraw(A).await;
    t.verify_bal_a(bal_a_after_awdraw).await;
    t.verify_bal_contract(bal_contract_after_awdraw).await;

    // B withdraws his funds.
    t.withdraw(B).await;
    t.verify_bal_b(bal_b_after_bwdraw).await;
    t.verify_bal_contract(bal_contract_after_bwdraw).await;
}

#[tokio::test]
pub async fn test_malicious_dispute() {
    let one_withdrawer = false;
    let mixed_asset = false;

    let bal_contract_after_afund = vec![100, 150];
    let bal_contract_after_bfund = vec![300, 400];

    let bal_contract_after_fclose = vec![300, 400];
    let bal_contract_after_awdraw = vec![150, 350];
    let bal_contract_after_bwdraw = vec![0, 0];

    let bal_a_after_afund = vec![0, 0];
    let bal_a_after_awdraw = vec![150, 50];

    let bal_b_after_bfund = vec![0, 0];
    let bal_b_after_bwdraw = vec![150, 350];

    let to_send_bal_first = vec![50, 0];
    let to_send_bal_second = vec![0, 100];

    // Setup the test environment
    let bal_a = vec![100, 150];
    let bal_b = vec![200, 250];
    let mut t = setup(10, bal_a, bal_b, mixed_asset, one_withdrawer)
        .await
        .expect("Failed to setup test");

    // Alice opens the channel.
    t.open().await;
    t.verify_state(&t.state).await;

    // Alice and Bob fund the channel.
    t.fund(A).await;
    t.verify_bal_contract(bal_contract_after_afund).await;
    t.verify_bal_a(bal_a_after_afund).await;

    t.fund(B).await;
    t.verify_bal_contract(bal_contract_after_bfund).await;
    t.verify_bal_b(bal_b_after_bfund).await;

    // Update channel off-chain.
    t.send_to_a(to_send_bal_first);

    // Bob save the state and signs it off-chain.
    let sig_a_cc = t.sigs_cc_abi_a();
    let sig_b_cc = t.sigs_cc_abi_b();
    let old_state = t.state.clone();

    // Alice sends a second off-chain update.
    t.send_to_b(to_send_bal_second);

    // Bob disputes the channel with the old state.
    t.dispute(B, old_state.clone(), sig_a_cc.clone(), sig_b_cc.clone())
        .await;
    t.verify_state(&old_state).await;

    // A disputes the channel with the new state.
    let sig_a_cc = t.sigs_cc_abi_a();
    let sig_b_cc = t.sigs_cc_abi_b();
    t.dispute(A, t.state.clone(), sig_a_cc.clone(), sig_b_cc.clone())
        .await;
    t.verify_state(&t.state).await;

    t.advance_clock_by(15_000).await; // Forward the clock to allow for dispute resolution.
                                      // A force-closes the channel.
    t.force_close(A).await;
    t.verify_state(&t.state).await;
    t.verify_bal_contract(bal_contract_after_fclose).await;

    // A withdraws her funds.
    t.withdraw(A).await;
    t.verify_bal_a(bal_a_after_awdraw).await;
    t.verify_bal_contract(bal_contract_after_awdraw).await;

    // B withdraws his funds.
    t.withdraw(B).await;
    t.verify_bal_b(bal_b_after_bwdraw).await;
    t.verify_bal_contract(bal_contract_after_bwdraw).await;
}
