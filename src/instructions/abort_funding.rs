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
            multi::Chain,
            perun_types::{Channel, ChannelID},
        },
    },
    borsh::BorshDeserialize,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        system_program,
        sysvar::Sysvar,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::instruction as token_instruction,
};

pub fn process_abort_funding(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    channel_id: ChannelID,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let channel_account = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    // 1. Get the channel PDA.
    let (channel_pda, bump) = Pubkey::find_program_address(
        &[Channel::SEED_PREFIX.as_bytes(), channel_id.as_bytes()],
        program_id,
    );

    if channel_account.key != &channel_pda {
        msg!("Wrong PDA passed");
        return Err(ProgramError::InvalidArgument);
    }

    // 2. Verify the channel state.
    let channel = Channel::try_from_slice(&channel_account.try_borrow_data()?)?;
    if channel.is_funded() {
        msg!("Fully funded channel cannot be aborted");
        return Err(PerunError::AbortFundingOnFundedChannel.into());
    }

    if channel.control.closed {
        msg!("Channel is already closed");
        return Err(PerunError::AbortFundingOnClosedChannel.into());
    }
    if channel.control.disputed {
        msg!("Channel is already disputed");
        return Err(PerunError::AbortFundingOnDisputedChannel.into());
    }

    if !channel.control.funded_a && !channel.control.funded_b {
        msg!("Channel is not funded by either party");
        return Err(PerunError::AbortFundingWithoutFunds.into());
    }

    // At this point we know that exactly one party has funded the channel.
    // Now we identify that party.
    let (actor, amount) = match channel.control.funded_a {
        true => (
            channel.params.a.solana_address.clone(),
            channel.state.balances.bal_a.clone(),
        ),
        false => (
            channel.params.b.solana_address.clone(),
            channel.state.balances.bal_b.clone(),
        ),
    };

    // Authenticate the actor.
    if payer.key != &actor {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // 3. Transfer the funds back to the actor.
    let tokens = &channel.state.balances.tokens;
    // Transfer lamports from channel PDA to the actor.
    for i in 0..tokens.len() {
        let token = tokens.get(i).unwrap();
        if token.chain == Chain::new(Chain::SOLANA_BACKEND_ID) {
            if amount[i] > 0 {
                if token.is_native_sol() {
                    // Native SOL transfer.
                    let lamports = amount[i] as u64;
                    **channel_account.try_borrow_mut_lamports()? = channel_account
                        .lamports()
                        .checked_sub(lamports)
                        .ok_or(ProgramError::InsufficientFunds)?;

                    **payer.try_borrow_mut_lamports()? = payer
                        .lamports()
                        .checked_add(lamports)
                        .ok_or(ProgramError::InsufficientFunds)?;
                } else {
                    // SPL token transfer
                    let mint_account = next_account_info(account_info_iter)?;
                    let to_associated_token_account = next_account_info(account_info_iter)?;
                    let channel_associated_token_account = next_account_info(account_info_iter)?;
                    let system_program = next_account_info(account_info_iter)?;
                    let token_program = next_account_info(account_info_iter)?;
                    let associated_token_program = next_account_info(account_info_iter)?;
                    if to_associated_token_account.lamports() == 0 {
                        msg!(
                            "Creating associated token account for receiver: {}",
                            to_associated_token_account.key
                        );
                        // Creating associated token account for recipient.
                        invoke(
                            &associated_token_account_instruction::create_associated_token_account(
                                payer.key,
                                payer.key,
                                mint_account.key,
                                token_program.key,
                            ),
                            &[
                                mint_account.clone(),
                                to_associated_token_account.clone(),
                                payer.clone(),
                                system_program.clone(),
                                token_program.clone(),
                                associated_token_program.clone(),
                            ],
                        )?;
                    }
                    msg!(
                        "Recipient Associated Token Address: {}",
                        to_associated_token_account.key
                    );

                    let token_amount = amount[i] as u64;
                    // Seeds for channel PDA signing
                    let seeds = &[
                        Channel::SEED_PREFIX.as_bytes(),
                        channel_id.as_bytes(),
                        &[bump],
                    ];
                    invoke_signed(
                        &token_instruction::transfer(
                            token_program.key,
                            channel_associated_token_account.key,
                            to_associated_token_account.key,
                            &channel_pda,
                            &[],
                            token_amount,
                        )?,
                        &[
                            channel_associated_token_account.clone(),
                            to_associated_token_account.clone(),
                            channel_account.clone(),
                            token_program.clone(),
                        ],
                        &[seeds],
                    )?
                }
            }
        }
    }

    // 4. Send the rent back to the creator of the channel account.
    msg!("Returning rent to the creator");
    let creator_account = next_account_info(account_info_iter)?;
    let rent = Rent::get()?;
    let channel_span = borsh::to_vec(&channel).unwrap().len();
    let required_lamports = rent.minimum_balance(channel_span);
    if &channel.control.creator != creator_account.key {
        msg!("Only the original creator can receive the rent");
        return Err(ProgramError::IllegalOwner);
    }
    **channel_account.try_borrow_mut_lamports()? = channel_account
        .lamports()
        .checked_sub(required_lamports)
        .ok_or(ProgramError::InsufficientFunds)?;
    **creator_account.try_borrow_mut_lamports()? = creator_account
        .lamports()
        .checked_add(required_lamports)
        .ok_or(ProgramError::InsufficientFunds)?;

    // 5. Close the channel account.
    let mut data = channel_account.try_borrow_mut_data()?;
    data.fill(0);
    channel_account.assign(&system_program::ID);

    // 6. Emit abort funding event.
    msg!(
        "Event: perun:abort_funding: channel: {:?}, actor: {:?}",
        channel_id,
        actor,
    );
    Ok(())
}
