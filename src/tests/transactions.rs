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
    crate::tests::setup::Test,
    crate::{
        instructions::perun_instructions::PerunInstruction,
        state::{Channel, ChannelState},
    },
    alloy_primitives::keccak256,
    alloy_sol_types::SolValue,
    solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        secp256k1_program,
        signature::Signer,
        signer::keypair::Keypair,
        system_program,
        transaction::Transaction,
    },
    spl_associated_token_account::get_associated_token_address_with_program_id,
};

impl Test {
    pub async fn open(&mut self) {
        // Derive channel PDA
        let (channel_pda, _bump) = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        );

        // Serialize open instruction
        let open_ix = Instruction::new_with_borsh(
            self.program_id,
            &PerunInstruction::Open {
                params: self.params.clone(),
                state: self.state.clone(),
            },
            vec![
                AccountMeta::new(channel_pda, false),
                AccountMeta::new(self.alice.solana_address, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );
        // Create and send the transaction
        let tx = Transaction::new_signed_with_payer(
            &[open_ix],
            Some(&self.alice.solana_address),
            &[&self.alice_keypair.solana_signer],
            self.program_test_ctx
                .banks_client
                .get_latest_blockhash()
                .await
                .unwrap(),
        );
        self.program_test_ctx
            .banks_client
            .process_transaction(tx)
            .await
            .expect("Failed to process open transaction");
    }

    pub async fn fund(&mut self, party_idx: bool) {
        // Derive channel PDA
        let (channel_pda, _bump) = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        );

        // Create channel associated token account
        let cata_0 = get_associated_token_address_with_program_id(
            &channel_pda,
            self.token_addresses.get(0).unwrap(),
            &spl_token::id(),
        );

        let mixed_assets = self.mixed_assets;

        let actor;
        let actor_kp: &Keypair;
        let fund_ix;
        if !mixed_assets {
            // If mixed assets, create associated token account for the second token
            let cata_1 = get_associated_token_address_with_program_id(
                &channel_pda,
                self.token_addresses.get(1).unwrap(),
                &spl_token::id(),
            );

            let actor_ata_0;
            let actor_ata_1;
            if party_idx {
                // Fund for party B
                actor = self.bob.solana_address;
                actor_kp = &self.bob_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
                actor_ata_1 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(1).unwrap(),
                    &spl_token::id(),
                );
            } else {
                // Fund for party A
                actor = self.alice.solana_address;
                actor_kp = &self.alice_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
                actor_ata_1 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(1).unwrap(),
                    &spl_token::id(),
                );
            }

            // Serialize fund instruction
            fund_ix = Instruction::new_with_borsh(
                self.program_id,
                &PerunInstruction::Fund {
                    channel_id: self.channel_id.clone(),
                    party_idx,
                },
                vec![
                    AccountMeta::new(channel_pda, false),
                    AccountMeta::new(actor, true),
                    AccountMeta::new_readonly(system_program::id(), false),
                    // For token 0
                    AccountMeta::new(self.token_addresses.get(0).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_0, false),
                    AccountMeta::new(cata_0, false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                    // For token 1
                    AccountMeta::new(self.token_addresses.get(1).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_1, false),
                    AccountMeta::new(cata_1, false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                ],
            );
        } else {
            let actor_ata_0;

            if party_idx {
                // Fund for party B
                actor = self.bob.solana_address;
                actor_kp = &self.bob_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
            } else {
                // Fund for party A
                actor = self.alice.solana_address;
                actor_kp = &self.alice_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
            }

            // Serialize fund instruction
            fund_ix = Instruction::new_with_borsh(
                self.program_id,
                &PerunInstruction::Fund {
                    channel_id: self.channel_id.clone(),
                    party_idx,
                },
                vec![
                    AccountMeta::new(channel_pda, false),
                    AccountMeta::new(actor, true),
                    AccountMeta::new_readonly(system_program::id(), false),
                    // For token 0
                    AccountMeta::new(self.token_addresses.get(0).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_0, false),
                    AccountMeta::new(cata_0, false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                ],
            );
        }
        // Create and send the transaction
        let tx = Transaction::new_signed_with_payer(
            &[fund_ix],
            Some(&actor),
            &[actor_kp],
            self.program_test_ctx
                .banks_client
                .get_latest_blockhash()
                .await
                .unwrap(),
        );
        self.program_test_ctx
            .banks_client
            .process_transaction(tx)
            .await
            .expect("Failed to process fund transaction");
    }

    pub async fn close(&mut self, state: ChannelState, sig_a: [u8; 65], sig_b: [u8; 65]) {
        // Derive channel PDA
        let (channel_pda, _bump) = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        );

        // Serialize close instruction
        let close_ix = Instruction::new_with_borsh(
            self.program_id,
            &PerunInstruction::Close {
                state,
                sig_a,
                sig_b,
            },
            vec![AccountMeta::new(channel_pda, false)],
        );

        // Create and send the transaction
        let tx = Transaction::new_signed_with_payer(
            &[close_ix],
            Some(&self.alice.solana_address),
            &[&self.alice_keypair.solana_signer],
            self.program_test_ctx
                .banks_client
                .get_latest_blockhash()
                .await
                .unwrap(),
        );
        self.program_test_ctx
            .banks_client
            .process_transaction(tx)
            .await
            .expect("Failed to process close transaction");
    }

    pub async fn withdraw(&mut self, party_idx: bool) {
        // Derive channel PDA
        let (channel_pda, _bump) = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        );

        // Create channel associated token account
        let cata_0 = get_associated_token_address_with_program_id(
            &channel_pda,
            self.token_addresses.get(0).unwrap(),
            &spl_token::id(),
        );

        let mixed_assets = self.mixed_assets;
        let one_withdrawer = self.one_withdrawer;

        let actor;
        let actor_kp: &Keypair;
        let actor_ata_0;
        let withdraw_ix;
        if !mixed_assets {
            let cata_1 = get_associated_token_address_with_program_id(
                &channel_pda,
                self.token_addresses.get(1).unwrap(),
                &spl_token::id(),
            );

            let actor_ata_1;
            if party_idx {
                // Withdraw for party B
                actor = self.bob.solana_address;
                actor_kp = &self.bob_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
                actor_ata_1 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(1).unwrap(),
                    &spl_token::id(),
                );
            } else {
                // Withdraw for party A
                actor = self.alice.solana_address;
                actor_kp = &self.alice_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
                actor_ata_1 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(1).unwrap(),
                    &spl_token::id(),
                );
            }

            // Serialize withdraw instruction
            withdraw_ix = Instruction::new_with_borsh(
                self.program_id,
                &PerunInstruction::Withdraw {
                    channel_id: self.channel_id.clone(),
                    party_idx,
                    one_withdrawer,
                },
                vec![
                    AccountMeta::new(channel_pda, false),
                    AccountMeta::new(actor, true),
                    // For token 0
                    AccountMeta::new(self.token_addresses.get(0).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_0, false),
                    AccountMeta::new(cata_0, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                    // For token 1
                    AccountMeta::new(self.token_addresses.get(1).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_1, false),
                    AccountMeta::new(cata_1, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                    // Channel Creator
                    AccountMeta::new(self.alice.solana_address, false),
                ],
            );
        } else {
            if party_idx {
                // Withdraw for party B
                actor = self.bob.solana_address;
                actor_kp = &self.bob_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
            } else {
                // Withdraw for party A
                actor = self.alice.solana_address;
                actor_kp = &self.alice_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
            }

            // Serialize withdraw instruction
            withdraw_ix = Instruction::new_with_borsh(
                self.program_id,
                &PerunInstruction::Withdraw {
                    channel_id: self.channel_id.clone(),
                    party_idx,
                    one_withdrawer,
                },
                vec![
                    AccountMeta::new(channel_pda, false),
                    AccountMeta::new(actor, true),
                    // For token 0
                    AccountMeta::new(self.token_addresses.get(0).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_0, false),
                    AccountMeta::new(cata_0, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                    // Channel Creator
                    AccountMeta::new(self.alice.solana_address, false),
                ],
            );
        }
        let tx = Transaction::new_signed_with_payer(
            &[withdraw_ix],
            Some(&actor),
            &[actor_kp],
            self.program_test_ctx
                .banks_client
                .get_latest_blockhash()
                .await
                .unwrap(),
        );
        self.program_test_ctx
            .banks_client
            .process_transaction(tx)
            .await
            .expect("Failed to process withdraw transaction");
    }

    pub async fn abort(&mut self, party_idx: bool) {
        // Derive channel PDA
        let (channel_pda, _bump) = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        );

        // Create channel associated token account
        // Create channel associated token account
        let cata_0 = get_associated_token_address_with_program_id(
            &channel_pda,
            self.token_addresses.get(0).unwrap(),
            &spl_token::id(),
        );

        let mixed_assets = self.mixed_assets;

        let actor;
        let actor_kp: &Keypair;
        let actor_ata_0;
        let abort_ix;
        if !mixed_assets {
            let cata_1 = get_associated_token_address_with_program_id(
                &channel_pda,
                self.token_addresses.get(1).unwrap(),
                &spl_token::id(),
            );

            let actor_ata_1;
            if party_idx {
                // Abort funding for party B
                actor = self.bob.solana_address;
                actor_kp = &self.bob_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
                actor_ata_1 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(1).unwrap(),
                    &spl_token::id(),
                );
            } else {
                // Abort funding for party A
                actor = self.alice.solana_address;
                actor_kp = &self.alice_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
                actor_ata_1 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(1).unwrap(),
                    &spl_token::id(),
                );
            }

            // Serialize abort instruction
            abort_ix = Instruction::new_with_borsh(
                self.program_id,
                &PerunInstruction::AbortFunding {
                    channel_id: self.channel_id.clone(),
                },
                vec![
                    AccountMeta::new(channel_pda, false),
                    AccountMeta::new(actor, true),
                    // For token 0
                    AccountMeta::new(self.token_addresses.get(0).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_0, false),
                    AccountMeta::new(cata_0, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                    // For token 1
                    AccountMeta::new(self.token_addresses.get(1).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_1, false),
                    AccountMeta::new(cata_1, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                    // Channel Creator
                    AccountMeta::new(self.alice.solana_address, false),
                ],
            );
        } else {
            if party_idx {
                // Abort funding for party B
                actor = self.bob.solana_address;
                actor_kp = &self.bob_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.bob.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
            } else {
                // Abort funding for party A
                actor = self.alice.solana_address;
                actor_kp = &self.alice_keypair.solana_signer;
                actor_ata_0 = get_associated_token_address_with_program_id(
                    &self.alice.solana_address,
                    self.token_addresses.get(0).unwrap(),
                    &spl_token::id(),
                );
            }

            // Serialize withdraw instruction
            abort_ix = Instruction::new_with_borsh(
                self.program_id,
                &PerunInstruction::AbortFunding {
                    channel_id: self.channel_id.clone(),
                },
                vec![
                    AccountMeta::new(channel_pda, false),
                    AccountMeta::new(actor, true),
                    // For token 0
                    AccountMeta::new(self.token_addresses.get(0).unwrap().clone(), false),
                    AccountMeta::new(actor_ata_0, false),
                    AccountMeta::new(cata_0, false),
                    AccountMeta::new_readonly(system_program::id(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                    // Channel Creator
                    AccountMeta::new(self.alice.solana_address, false),
                ],
            );
        }

        // Create and send the transaction
        let tx = Transaction::new_signed_with_payer(
            &[abort_ix],
            Some(&actor),
            &[&actor_kp],
            self.program_test_ctx
                .banks_client
                .get_latest_blockhash()
                .await
                .unwrap(),
        );
        self.program_test_ctx
            .banks_client
            .process_transaction(tx)
            .await
            .expect("Failed to process abort transaction");
    }

    pub async fn dispute(
        &mut self,
        party_idx: bool,
        state: ChannelState,
        sig_a: [u8; 65],
        sig_b: [u8; 65],
    ) {
        // Derive channel PDA
        let (channel_pda, _bump) = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        );

        // Serialize close instruction
        let dispute_ix = Instruction::new_with_borsh(
            self.program_id,
            &PerunInstruction::Dispute {
                state,
                sig_a,
                sig_b,
            },
            vec![AccountMeta::new(channel_pda, false)],
        );

        let actor;
        let actor_keypair: &Keypair;
        if party_idx {
            // If party B is disputing, use Bob's address
            actor = self.bob.solana_address;
            actor_keypair = &self.bob_keypair.solana_signer;
        } else {
            // If party A is disputing, use Alice's address
            actor = self.alice.solana_address;
            actor_keypair = &self.alice_keypair.solana_signer;
        }

        // Create and send the transaction
        let tx = Transaction::new_signed_with_payer(
            &[dispute_ix],
            Some(&actor),
            &[&actor_keypair],
            self.program_test_ctx
                .banks_client
                .get_latest_blockhash()
                .await
                .unwrap(),
        );
        self.program_test_ctx
            .banks_client
            .process_transaction(tx)
            .await
            .expect("Failed to process close transaction");
    }

    pub async fn force_close(&mut self, party_idx: bool) {
        // Derive channel PDA
        let (channel_pda, _bump) = Pubkey::find_program_address(
            &[Channel::SEED_PREFIX.as_bytes(), self.channel_id.as_bytes()],
            &self.program_id,
        );

        // Serialize force close instruction
        let force_close_ix = Instruction::new_with_borsh(
            self.program_id,
            &PerunInstruction::ForceClose {
                channel_id: self.channel_id.clone(),
            },
            vec![AccountMeta::new(channel_pda, false)],
        );

        let actor;
        let actor_keypair: &Keypair;
        if party_idx {
            // If party B is disputing, use Bob's address
            actor = self.bob.solana_address;
            actor_keypair = &self.bob_keypair.solana_signer;
        } else {
            // If party A is disputing, use Alice's address
            actor = self.alice.solana_address;
            actor_keypair = &self.alice_keypair.solana_signer;
        }

        // Create and send the transaction
        let tx = Transaction::new_signed_with_payer(
            &[force_close_ix],
            Some(&actor),
            &[&actor_keypair],
            self.program_test_ctx
                .banks_client
                .get_latest_blockhash()
                .await
                .unwrap(),
        );
        self.program_test_ctx
            .banks_client
            .process_transaction(tx)
            .await
            .expect("Failed to process force close transaction");
    }
}
