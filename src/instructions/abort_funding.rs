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
        account_info::{AccountInfo, next_account_info},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        system_program,
        sysvar::Sysvar,
    },
};

pub fn process_abort_funding(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    channel_id: ChannelID,
) -> ProgramResult {
    msg!(
        "Processing AbortFunding instruction with program_id: {:?}, channel_id: {:?}",
        program_id,
        channel_id
    );

    let account_info_iter = &mut accounts.iter();
    let channel_account = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let creator_account = next_account_info(account_info_iter)?;
    // 1. Get the channel PDA.
    let (channel_pda, _bump) = Pubkey::find_program_address(
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
                let lamports = amount[i] as u64;
                channel_account
                    .try_borrow_mut_lamports()?
                    .checked_sub(lamports)
                    .ok_or(ProgramError::InsufficientFunds)?;
                payer
                    .try_borrow_mut_lamports()?
                    .checked_add(lamports)
                    .ok_or(ProgramError::InsufficientFunds)?;
            }
        }
    }

    // 4. Send the rent back to the creator of the channel account.
    let rent = Rent::get()?;
    let channel_span = channel.get_size();
    let required_lamports = rent.minimum_balance(channel_span);
    if &channel.control.creator != creator_account.key {
        msg!("Only the original creator can receive the rent");
        return Err(ProgramError::IllegalOwner);
    }

    creator_account
        .try_borrow_mut_lamports()?
        .checked_add(required_lamports)
        .ok_or(ProgramError::InsufficientFunds)?;
    channel_account
        .try_borrow_mut_lamports()?
        .checked_sub(required_lamports)
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
