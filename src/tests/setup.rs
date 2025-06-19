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

use {
    crate::{
        entrypoint::process_instruction,
        instructions::perun_instructions::PerunInstruction,
        state::{
            ethsig::EthSigner,
            multi::{Chain, ChannelPubKeyCross, CrossAsset},
            sol::get_channel_id_cross,
            Balances, Channel, ChannelID, ChannelState, Params, Participant,
        },
        tests::helpers::*,
    },
    alloy_primitives::{address, Address as EthAddr},
    borsh::BorshDeserialize,
    rand::Rng,
    solana_program::program_pack::Pack,
    solana_program_test::*,
    solana_sdk::{
        account::Account as SolanaAccount, msg, pubkey::Pubkey, signature::Signer,
        signer::keypair::Keypair, system_instruction, sysvar::clock::Clock, sysvar::rent::Rent,
        transaction::Transaction, transport::TransportError,
    },
    spl_associated_token_account::{
        get_associated_token_address_with_program_id,
        instruction as associated_token_account_instruction,
    },
    spl_token::state::{Account, Mint},
};
pub struct Test {
    pub program_test_ctx: ProgramTestContext,
    pub program_id: Pubkey,
    pub alice: Participant,
    pub bob: Participant,
    pub alice_keypair: TestKeyPair,
    pub bob_keypair: TestKeyPair,
    pub params: Params,
    pub channel_id: ChannelID,
    pub state: ChannelState,
    pub token_addresses: Vec<Pubkey>,
    pub mixed_assets: bool,
    pub one_withdrawer: bool,
}

impl Test {
    pub async fn verify_state(&self, expected_state: &ChannelState) {
        // Fetch the channel account
        let channel_pda = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        )
        .0;

        let channel_account = self
            .program_test_ctx
            .banks_client
            .get_account(channel_pda)
            .await
            .expect("Failed to get channel account")
            .expect("Channel account not found");

        // Deserialize the channel state
        let channel = Channel::try_from_slice(&channel_account.data)
            .expect("Failed to deserialize channel state");

        assert_eq!(channel.state, *expected_state, "Channel state mismatch");
    }

    pub async fn verify_bal_a(&self, expected_balances: Vec<u64>) {
        self.verfy_bal(&self.alice.solana_address, expected_balances)
            .await;
    }

    pub async fn verify_bal_b(&self, expected_balances: Vec<u64>) {
        self.verfy_bal(&self.bob.solana_address, expected_balances)
            .await;
    }

    pub async fn verify_bal_contract(&self, bal: Vec<u64>) {
        let (channel_pda, _bump) = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        );

        for i in 0..self.token_addresses.len() {
            let cata = get_associated_token_address_with_program_id(
                &channel_pda,
                &self.token_addresses[i],
                &spl_token::id(),
            );

            let account = self
                .program_test_ctx
                .banks_client
                .get_account(cata)
                .await
                .expect("Failed to get channel associated token account")
                .expect("Channel associated token account not found");

            let token_account =
                Account::unpack(&account.data).expect("Failed to unpack token account data");

            assert_eq!(
                token_account.amount, bal[i],
                "Token balance mismatch for asset {}",
                i
            );
        }
    }

    pub async fn verfy_bal(&self, participant_pubkey: &Pubkey, expected_balances: Vec<u64>) {
        for i in 0..self.token_addresses.len() {
            let ata = get_associated_token_address_with_program_id(
                participant_pubkey,
                &self.token_addresses[i],
                &spl_token::id(),
            );

            let account = self
                .program_test_ctx
                .banks_client
                .get_account(ata)
                .await
                .expect("Failed to get associated token account")
                .expect("Associated token account not found");

            let token_account =
                Account::unpack(&account.data).expect("Failed to unpack token account data");

            assert_eq!(
                token_account.amount, expected_balances[i],
                "Token balance mismatch for asset {}",
                i
            );
        }
    }

    pub fn update(&mut self, new_state: ChannelState) {
        self.state = new_state;
    }

    pub fn send_to_a(&mut self, amt: Vec<u64>) {
        assert_eq!(
            self.state.balances.bal_a.len(),
            amt.len(),
            "length of bal_a and amt must be the same"
        );
        assert_eq!(
            self.state.balances.bal_b.len(),
            amt.len(),
            "length of bal_b and amt must be the same"
        );

        let mut new_bal_a = Vec::new();
        let mut new_bal_b = Vec::new();
        for i in 0..amt.len() {
            let bal_a = self.state.balances.bal_a.get(i).unwrap() + amt.get(i).unwrap();
            let bal_b = self.state.balances.bal_b.get(i).unwrap() - amt.get(i).unwrap();
            new_bal_a.push(bal_a);
            new_bal_b.push(bal_b);
        }

        self.update(ChannelState {
            channel_id: self.state.channel_id.clone(),
            balances: Balances {
                tokens: self.state.balances.tokens.clone(),
                bal_a: new_bal_a,
                bal_b: new_bal_b,
            },
            version: self.state.version + 1,
            finalized: self.state.finalized,
        })
    }

    pub fn send_to_b(&mut self, amt: Vec<u64>) {
        assert_eq!(
            self.state.balances.bal_a.len(),
            amt.len(),
            "length of bal_a and amt must be the same"
        );
        assert_eq!(
            self.state.balances.bal_b.len(),
            amt.len(),
            "length of bal_b and amt must be the same"
        );

        let mut new_bal_a = Vec::new();
        let mut new_bal_b = Vec::new();
        for i in 0..amt.len() {
            let bal_a = self.state.balances.bal_a.get(i).unwrap() - amt.get(i).unwrap();
            let bal_b = self.state.balances.bal_b.get(i).unwrap() + amt.get(i).unwrap();
            new_bal_a.push(bal_a);
            new_bal_b.push(bal_b);
        }

        self.update(ChannelState {
            channel_id: self.state.channel_id.clone(),
            balances: Balances {
                tokens: self.state.balances.tokens.clone(),
                bal_a: new_bal_a,
                bal_b: new_bal_b,
            },
            version: self.state.version + 1,
            finalized: self.state.finalized,
        })
    }

    pub fn finalize(&mut self) {
        self.update(ChannelState {
            version: self.state.version + 1,
            finalized: true,
            ..self.state.clone()
        });
    }

    pub fn sigs_cc_abi_a(&self) -> [u8; 65] {
        sign_cross_abi(&self.alice_keypair, &self.state)
    }

    pub fn sigs_cc_abi_b(&self) -> [u8; 65] {
        sign_cross_abi(&self.bob_keypair, &self.state)
    }

    pub async fn advance_clock_by(&mut self, ms: i64) {
        let clock: Clock = self
            .program_test_ctx
            .banks_client
            .get_sysvar()
            .await
            .expect("get Clock sysvar");
        let start_ts = clock.unix_timestamp;
        let start_slot = clock.slot;

        let slots_needed = (ms as f64).ceil() as u64;
        let target_slot = start_slot + slots_needed;

        // Warp to target slot
        self.program_test_ctx.warp_to_slot(target_slot).unwrap();

        let clock2: Clock = self
            .program_test_ctx
            .banks_client
            .get_sysvar()
            .await
            .expect("get Clock sysvar");
        msg!(
            "Advanced time from ts={} (slot={}) to ts={} (slot={})",
            start_ts,
            start_slot,
            clock2.unix_timestamp,
            clock2.slot
        );
    }
}

pub async fn setup(
    challenge_duration: u64,
    bal_a: Vec<u64>,
    bal_b: Vec<u64>,
    mixed_assets: bool,
    one_withdrawer: bool,
) -> Result<Test, TransportError> {
    let program_id = Pubkey::new_unique();
    // Solana addresses for Alice and Bob
    let alice_solana = Keypair::new();
    let bob_solana = Keypair::new();

    let mut program_test = ProgramTest::new(
        "perun_solana_program",
        program_id,
        processor!(process_instruction),
    );

    program_test.add_account(
        alice_solana.pubkey(),
        SolanaAccount {
            lamports: 1_000_000_000, // 1 SOL
            ..SolanaAccount::default()
        },
    );
    program_test.add_account(
        bob_solana.pubkey(),
        SolanaAccount {
            lamports: 1_000_000_000, // 1 SOL
            ..SolanaAccount::default()
        },
    );

    // Start the program test
    let mut ctx = program_test.start_with_context().await;

    let (alice, bob, alice_keypair, bob_keypair) = {
        let (alice_privkey, alice_pubkey) = generate_secp_keypair();
        let (bob_privkey, bob_pubkey) = generate_secp_keypair();

        let alice_keypair = EthSigner::init_from_key(alice_privkey);
        let bob_keypair = EthSigner::init_from_key(bob_privkey);

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

        let alice_keypair = TestKeyPair {
            eth_signer: alice_keypair,
            solana_signer: alice_solana,
        };
        let bob_keypair = TestKeyPair {
            eth_signer: bob_keypair,
            solana_signer: bob_solana,
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

    // Create and fund the associated token accounts for Alice and Bob.
    setup_and_fund_token_accounts(
        &mut ctx,
        &alice,
        bal_a[0].try_into().unwrap(),
        &bob,
        bal_b[0].try_into().unwrap(),
        &mint_0,
    )
    .await?;

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

        // Create and fund the associated token accounts for Alice and Bob.
        setup_and_fund_token_accounts(
            &mut ctx,
            &alice,
            bal_a[1].try_into().unwrap(),
            &bob,
            bal_b[1].try_into().unwrap(),
            &mint_1,
        )
        .await?;

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
        program_id,
        alice,
        bob,
        alice_keypair,
        bob_keypair,
        params,
        channel_id,
        state,
        token_addresses,
        mixed_assets,
        one_withdrawer, // Default to false, can be set later
    })
}

async fn setup_and_fund_token_accounts(
    ctx: &mut ProgramTestContext,
    alice: &Participant,
    bal_a: u64,
    bob: &Participant,
    bal_b: u64,
    mint: &Keypair,
) -> Result<(), TransportError> {
    // Define associated token accounts for Alice and Bob.
    let ata_a = get_associated_token_address_with_program_id(
        &alice.solana_address,
        &mint.pubkey(),
        &spl_token::id(),
    );
    let ata_b = get_associated_token_address_with_program_id(
        &bob.solana_address,
        &mint.pubkey(),
        &spl_token::id(),
    );

    // Create associated token accounts for Alice and Bob.
    let ata_tx_a = Transaction::new_signed_with_payer(
        &[
            associated_token_account_instruction::create_associated_token_account(
                &ctx.payer.pubkey(),
                &alice.solana_address,
                &mint.pubkey(),
                &spl_token::id(),
            ),
        ],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.banks_client.get_latest_blockhash().await.unwrap(),
    );
    ctx.banks_client.process_transaction(ata_tx_a).await?;

    let ata_tx_b = Transaction::new_signed_with_payer(
        &[
            associated_token_account_instruction::create_associated_token_account(
                &ctx.payer.pubkey(),
                &bob.solana_address,
                &mint.pubkey(),
                &spl_token::id(),
            ),
        ],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.banks_client.get_latest_blockhash().await.unwrap(),
    );
    ctx.banks_client.process_transaction(ata_tx_b).await?;

    // Fund the associated token accounts with some tokens.
    let fund_tx_a = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint.pubkey(),
            &ata_a,
            &ctx.payer.pubkey(),
            &[&ctx.payer.pubkey()],
            bal_a, // Mint tokens to Alice
        )
        .unwrap()],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.banks_client.get_latest_blockhash().await.unwrap(),
    );
    ctx.banks_client.process_transaction(fund_tx_a).await?;

    let fund_tx_b = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint.pubkey(),
            &ata_b,
            &ctx.payer.pubkey(),
            &[&ctx.payer.pubkey()],
            bal_b, // Mint 1000 tokens to Bob
        )
        .unwrap()],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.banks_client.get_latest_blockhash().await.unwrap(),
    );
    ctx.banks_client.process_transaction(fund_tx_b).await?;

    // Verify the token accounts have been created and funded
    let alice_ata = ctx.banks_client.get_account(ata_a).await?.unwrap();
    let bob_ata = ctx.banks_client.get_account(ata_b).await?.unwrap();
    let alice_ata_data = Account::unpack(&alice_ata.data).expect("unpack alice ata");
    let bob_ata_data = Account::unpack(&bob_ata.data).expect("unpack bob ata");
    assert_eq!(alice_ata_data.amount, bal_a);
    assert_eq!(bob_ata_data.amount, bal_b);
    Ok(())
}
