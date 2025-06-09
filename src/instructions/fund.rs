//  Copyright 2025 PolyCrypt GmbH
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use {
    crate::{
        error::PerunError,
        state::{
            multi::{Chain, CrossAsset},
            perun_types::{Channel, ChannelID},
        },
    },
    borsh::BorshDeserialize,
    solana_program::{
        account_info::{AccountInfo, next_account_info},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction,
    },
};

const A: bool = false;
const B: bool = true;

/// fund funds a channel with the given channel_id. If party_idx is false, the funding
/// is provided on behalf of party A, if party_idx is true, the funding is provided on
/// behalf of party B.
pub fn process_fund(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    channel_id: ChannelID,
    party_idx: bool,
) -> ProgramResult {
    msg!(
        "Processing Fund instruction with program_id: {:?}, channel_id: {:?}, party_idx: {}",
        program_id,
        channel_id,
        party_idx
    );
    let account_info_iter = &mut accounts.iter();
    let channel_account = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // 1. Get the channel PDA
    let (channel_pda, _bump) = Pubkey::find_program_address(
        &[Channel::SEED_PREFIX.as_bytes(), channel_id.as_bytes()],
        program_id,
    );

    if channel_account.key != &channel_pda {
        msg!("Wrong PDA passed");
        return Err(ProgramError::InvalidArgument);
    }

    let mut channel = Channel::try_from_slice(&channel_account.try_borrow_mut_data()?)?;

    // 2. Fund the with the corresponding party index
    let (expected_funder, amount) = match party_idx {
        A => {
            // Fund for party A.
            // Verify that A has not yet been funded.
            if channel.control.funded_a {
                return Err(PerunError::AlreadyFunded.into());
            }

            // Set A's funded status to true.
            // Note that the transaction is rolled back, if funding fails at a later point,
            // so doing this now is not a problem.
            channel.control.funded_a = true;
            let other_funded = get_funded(
                channel.state.balances.tokens.clone(),
                channel.state.balances.bal_b.clone(),
            );
            if other_funded {
                channel.control.funded_b = true;
            }
            (
                channel.params.a.solana_address.clone(),
                channel.state.balances.bal_a.clone(),
            )
        }
        B => {
            // Fund for party B.
            if channel.control.funded_b {
                return Err(PerunError::AlreadyFunded.into());
            }
            channel.control.funded_b = true; // effect
            let other_funded = get_funded(
                channel.state.balances.tokens.clone(),
                channel.state.balances.bal_a.clone(),
            );
            if other_funded {
                channel.control.funded_a = true;
            }
            (
                channel.params.b.solana_address.clone(),
                channel.state.balances.bal_b.clone(),
            )
        }
    };

    // Verify signer is the expected funder
    if payer.key != &expected_funder {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // 3. Transfer the funds to the channel account.
    let tokens = &channel.state.balances.tokens;
    // Transfer lamports from payer to the channel PDA
    for i in 0..tokens.len() {
        let token = tokens.get(i).unwrap();
        if token.chain == Chain::new(Chain::SOLANA_BACKEND_ID) {
            if amount[i] > 0 {
                let lamports = amount[i] as u64;
                let transfer_ix =
                    system_instruction::transfer(payer.key, channel_account.key, lamports);
                invoke(
                    &transfer_ix,
                    &[
                        payer.clone(),
                        channel_account.clone(),
                        system_program.clone(),
                    ],
                )?;
            }
        }
    }

    // 4. Emit fund event.
    msg!(
        "Event: perun::fund channel {:?} for party {:?} with amount: {:?}",
        channel_id,
        if party_idx { "B" } else { "A" },
        amount
    );

    if channel.is_funded() {
        msg!("Event: perun::fund_c {:?} is fully funded", channel_id);
    }

    Ok(())
}

/// get_funded looks if other party has to fund
fn get_funded(tokens: Vec<CrossAsset>, amount: Vec<i128>) -> bool {
    let mut funded = true;
    for i in 0..tokens.len() {
        let token = tokens.get(i).unwrap();
        if token.chain == Chain::new(Chain::SOLANA_BACKEND_ID) {
            if let Some(amt) = amount.get(i) {
                if *amt > 0 {
                    funded = false
                }
            }
        }
    }
    return funded;
}
