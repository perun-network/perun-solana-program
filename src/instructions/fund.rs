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
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_token::instruction as token_instruction,
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
    // 2. Deserialize and mutate channel state in a limited scope
    let (expected_funder, amount, tokens) = {
        let mut data = channel_account.try_borrow_mut_data()?;
        let mut channel = Channel::try_from_slice(&data)?;

        let (expected_funder, amount) = match party_idx {
            A => {
                if channel.control.funded_a {
                    return Err(PerunError::AlreadyFunded.into());
                }
                channel.control.funded_a = true;
                let other_funded = get_funded(
                    channel.state.balances.tokens.as_slice(),
                    channel.state.balances.bal_b.as_slice(),
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
                if channel.control.funded_b {
                    return Err(PerunError::AlreadyFunded.into());
                }
                channel.control.funded_b = true;
                let other_funded = get_funded(
                    channel.state.balances.tokens.as_slice(),
                    channel.state.balances.bal_a.as_slice(),
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

        channel.serialize(&mut &mut data[..])?;

        // Return needed values and tokens reference (clone tokens vec for safety)
        (
            expected_funder,
            amount,
            channel.state.balances.tokens.clone(),
        )
    }; // `data` mutable borrow dropped here

    // 3. Verify signer is the expected funder
    if payer.key != &expected_funder {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // 3. Transfer the funds to the channel account.
    for i in 0..tokens.len() {
        let token = tokens.get(i).unwrap();
        if token.chain == Chain::new(Chain::SOLANA_BACKEND_ID) {
            if token.is_native_sol() {
                // lamport transfer
                if amount[i] > 0 {
                    let lamports = amount[i] as u64;
                    // Subtract lamports from payer
                    **payer.try_borrow_mut_lamports()? = payer
                        .lamports()
                        .checked_sub(lamports)
                        .ok_or(ProgramError::InsufficientFunds)?;

                    // Add lamports to channel account
                    **channel_account.try_borrow_mut_lamports()? = channel_account
                        .lamports()
                        .checked_add(lamports)
                        .ok_or(ProgramError::InsufficientFunds)?;
                }
            } else {
                // SPL token transfer
                let mint_account = next_account_info(account_info_iter)?;
                let from_associated_token_account = next_account_info(account_info_iter)?;
                let channel_associated_token_account = next_account_info(account_info_iter)?;
                let token_program = next_account_info(account_info_iter)?;
                let associated_token_program = next_account_info(account_info_iter)?;
                if amount[i] > 0 {
                    let token_amount = amount[i] as u64;
                    if channel_associated_token_account.lamports() == 0 {
                        msg!(
                            "Creating associated token account for channel: {}",
                            channel_account.key
                        );
                        // Create associated token account for the channel.
                        invoke(
                        &spl_associated_token_account::instruction::create_associated_token_account(
                            payer.key,
                            channel_account.key,
                            mint_account.key,
                            token_program.key,
                        ),
                        &[
                            mint_account.clone(),
                            channel_associated_token_account.clone(),
                            channel_account.clone(),
                            payer.clone(),
                            system_program.clone(),
                            token_program.clone(),
                            associated_token_program.clone(),
                        ],
                    )?;
                    }
                    msg!(
                        "Channel Associated Token Address: {}",
                        channel_associated_token_account.key
                    );
                    invoke(
                        &token_instruction::transfer(
                            token_program.key,
                            from_associated_token_account.key,
                            channel_associated_token_account.key,
                            payer.key,
                            &[payer.key],
                            token_amount,
                        )?,
                        &[
                            mint_account.clone(),
                            from_associated_token_account.clone(),
                            channel_associated_token_account.clone(),
                            payer.clone(),
                            token_program.clone(),
                        ],
                    )?;
                }
            }
        }
    }

    // 4. Emit fund event.
    msg!(
        "Event: perun:fund channel {:?} for party {:?}",
        channel_id,
        if party_idx { "B" } else { "A" },
    );

    Ok(())
}

/// get_funded looks if other party has to fund
fn get_funded(tokens: &[CrossAsset], amount: &[u64]) -> bool {
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
