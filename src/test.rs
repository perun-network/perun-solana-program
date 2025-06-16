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
use crate::{
    entrypoint::process_instruction,
    state::{
        ethsig::EthSigner,
        multi::{Chain, ChannelPubKeyCross, CrossAsset},
        sol::get_channel_id_cross,
        Balances, ChannelID, ChannelState, Params, Participant,
    },
};
use alloy_primitives::{address, keccak256, Address as EthAddr};
use alloy_sol_types::abi::token;
use k256::ecdsa::{SigningKey, VerifyingKey};
use rand::{thread_rng, Rng};
use solana_program::{program_pack::Pack, rent::Rent, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    address_lookup_table::program, instruction::Instruction, pubkey::Pubkey, signature::Signer,
    signer::keypair::Keypair, transaction::Transaction, transport::TransportError,
};
use spl_token::state::{Account, Mint};

#[tokio::test]
async fn test_perun_program() {
    let program_id = Pubkey::new_unique();
    let (mut banks_client, payer, recent_blockhash) =
        ProgramTest::new("perun", program_id, processor!(process_instruction))
            .start()
            .await;

    // Create a new keypair to use as the address for our counter account
    let counter_keypair = Keypair::new();
    let initial_value: u64 = 42;
}

struct Test {
    program_test_ctx: ProgramTestContext,
    alice: Participant,
    bob: Participant,
    alice_keypair: TestKeyPair,
    bob_keypair: TestKeyPair,
    params: Params,
    channel_id: ChannelID,
    state: ChannelState,
    token_addresses: Vec<Pubkey>,
}

pub async fn setup(
    challenge_duration: u64,
    bal_a: Vec<i128>,
    bal_b: Vec<i128>,
    mock_auth: bool,
    mixed_assets: bool,
) -> Result<Test, TransportError> {
    let program_id = Pubkey::new_unique();
    let program_test = ProgramTest::new(
        "perun-payment-channel",
        program_id,
        processor!(process_instruction),
    );

    // Start the program test
    let ctx = program_test.start_with_context().await;

    let (alice, bob, alice_keypair, bob_keypair) = {
        let (alice_privkey, alice_pubkey) = generate_secp_keypair();
        let (bob_privkey, bob_pubkey) = generate_secp_keypair();

        let alice_keypair = EthSigner::init_from_key(alice_privkey);
        let bob_keypair = EthSigner::init_from_key(bob_privkey);

        let alice_keypair = TestKeyPair {
            eth_signer: alice_keypair,
        };
        let bob_keypair = TestKeyPair {
            eth_signer: bob_keypair,
        };

        let alice_pubkey_bytes = get_pubkey_secp_bytes(&alice_pubkey);

        let bob_pubkey_bytes = get_pubkey_secp_bytes(&bob_pubkey);

        let alice_l2_pubkeys = ChannelPubKeyCross {
            key: alice_pubkey_bytes.clone(),
        };

        let bob_l2_pubkeys = ChannelPubKeyCross {
            key: bob_pubkey_bytes.clone(),
        };

        let alice_eth_bytes: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];

        let bob_eth_bytes: [u8; 20] = [
            21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
        ];

        // Solana addresses for Alice and Bob
        let alice_solana = Keypair::new();
        let bob_solana = Keypair::new();

        let alice = Participant {
            solana_address: alice_solana.pubkey(),
            cc_address: alice_eth_bytes,
            l2_pubkey: alice_l2_pubkeys.key,
        };

        let bob = Participant {
            solana_address: bob_solana.pubkey(),
            cc_address: bob_eth_bytes,
            l2_pubkey: bob_l2_pubkeys.key,
        };

        (alice, bob, alice_keypair, bob_keypair)
    };

    if bal_a.len() != 2 {
        panic!("test setup should utilize two assets")
    }

    if bal_a.len() != bal_b.len() {
        panic!("balances arrays are not of same length");
    }

    let mint_0 = Keypair::new();
    let decimals = 9;
    let rent = Rent::default();
    let mut token_addresses = vec![];
    token_addresses.push(mint_0.pubkey());
    // Setup the mint of token 0.
    let mint_transaction_0 = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &ctx.payer.pubkey(),
                &mint_0.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_0.pubkey(),
                &ctx.payer.pubkey(),
                None,
                decimals,
            )
            .unwrap(),
        ],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_0],
        ctx.banks_client.get_latest_blockhash().await.unwrap(),
    );
    ctx.banks_client
        .process_transaction(mint_transaction_0)
        .await
        .unwrap();

    let cross_assets_0;
    let cross_assets_1;

    if mixed_assets {
        cross_assets_0 = CrossAsset {
            solana_address: token_addresses.get(0).unwrap().clone(),
            chain: Chain::new(Chain::SOLANA_BACKEND_ID),
            eth_address: [0u8; 20],
        };
        let checksummed = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let expected = address!("d8da6bf26964af9d7eed9e03e53415d37aa96045");
        let eth_address = EthAddr::parse_checksummed(checksummed, None).expect("valid checksum");
        assert_eq!(eth_address, expected);

        let eth_addr_slice = eth_address.as_slice();
        let eth_addr_bytes: [u8; 20] = eth_addr_slice
            .try_into()
            .expect("ethereum address slice is not of length 20");

        cross_assets_1 = CrossAsset {
            solana_address: token_addresses.get(0).unwrap().clone(),
            eth_address: eth_addr_bytes,
            chain: Chain::new(1),
        };
    } else {
        // Setup the mint of token 1.
        let mint_1 = Keypair::new();
        token_addresses.push(mint_1.pubkey());
        let mint_transaction_1 = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &ctx.payer.pubkey(),
                    &mint_1.pubkey(),
                    rent.minimum_balance(Mint::LEN),
                    Mint::LEN as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_mint(
                    &spl_token::id(),
                    &mint_1.pubkey(),
                    &ctx.payer.pubkey(),
                    None,
                    decimals,
                )
                .unwrap(),
            ],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &mint_1],
            ctx.banks_client.get_latest_blockhash().await.unwrap(),
        );

        ctx.banks_client
            .process_transaction(mint_transaction_1)
            .await
            .unwrap();

        cross_assets_0 = CrossAsset {
            solana_address: token_addresses.get(0).unwrap().clone(),
            chain: Chain::new(Chain::SOLANA_BACKEND_ID),
            eth_address: [0u8; 20],
        };

        cross_assets_1 = CrossAsset {
            solana_address: token_addresses.get(1).unwrap().clone(),
            chain: Chain::new(Chain::SOLANA_BACKEND_ID),
            eth_address: [0u8; 20],
        };
    }

    let mut nonce = [0u8; 32];
    rand::thread_rng().fill(&mut nonce);

    let params = Params {
        a: alice.clone(),
        b: bob.clone(),
        nonce: nonce,
        challenge_duration: challenge_duration,
    };

    let channel_id = get_channel_id_cross(&params);
    let channel_id_bytes = channel_id.as_bytes();

    let state = ChannelState {
        channel_id: *channel_id_bytes,
        balances: Balances {
            tokens: vec![cross_assets_0, cross_assets_1], //vec![&e, cross_assets_1, cross_assets_2]
            bal_a,
            bal_b,
        },
        version: 0,
        finalized: false,
    };

    Ok(Test {
        program_test_ctx: ctx,
        alice,
        bob,
        alice_keypair,
        bob_keypair,
        params,
        channel_id,
        state,
        token_addresses,
    })
}

pub struct TestKeyPair {
    pub eth_signer: EthSigner, // Only the Ethereum signer
}

fn generate_secp_keypair() -> (SigningKey, VerifyingKey) {
    let privkey = SigningKey::random(&mut thread_rng());
    let pubkey = VerifyingKey::from(&privkey);
    (privkey, pubkey)
}

fn get_pubkey_secp_bytes(pubkey: &VerifyingKey) -> [u8; 65] {
    let pubkey_bytes: [u8; 65] = pubkey
        .to_encoded_point(false)
        .as_bytes()
        .try_into()
        .unwrap();
    pubkey_bytes
}
