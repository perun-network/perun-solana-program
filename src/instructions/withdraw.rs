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
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        system_program,
        sysvar::Sysvar,
    },
};

const A: bool = false;
const B: bool = true;

/// process_withdraw processes a withdraw instruction for a channel.
/// It withdraws a party's funds from a closed channel.
/// If the party_idx is false, withdraw is executed on behalf ob party A, else on behalf
/// of party B.
pub fn process_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    channel_id: ChannelID,
    party_idx: bool,
    one_withdrawer: bool,
) -> ProgramResult {
    msg!(
        "Processing Withdraw instruction with program_id: {:?}, channel_id: {:?}, party_idx: {}",
        program_id,
        channel_id,
        party_idx,
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
    let channel = &mut Channel::try_from_slice(&channel_account.try_borrow_data()?)?;
    if !channel.control.closed {
        msg!("Channel is not closed");
        return Err(PerunError::WithdrawOnOpenChannel.into());
    }

    // 3. Withdraw funds.
    // Determine the amount to withdraw based on the party index.
    let (amount, receiver) = match party_idx {
        A => {
            if channel.control.withdrawn_a {
                return Err(PerunError::AlreadyWithdrawn.into());
            }

            (
                channel.state.balances.bal_a.clone(),
                channel.params.a.solana_address.clone(),
            )
        }
        B => {
            if channel.control.withdrawn_b {
                return Err(PerunError::AlreadyWithdrawn.into());
            }
            (
                channel.state.balances.bal_b.clone(),
                channel.params.b.solana_address.clone(),
            )
        }
    };

    // Always authenticate as party B if oneWithdrawer is true
    let actor = if one_withdrawer {
        channel.params.b.solana_address.clone()
    } else {
        match party_idx {
            A => channel.params.a.solana_address.clone(),
            B => channel.params.b.solana_address.clone(),
        }
    };

    if payer.key != &actor {
        msg!("Payer is not the actor");
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Perform the withdrawal.
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
    // Update the control structure to mark the withdrawal.
    if party_idx == A {
        channel.control.withdrawn_a = true;
    } else {
        channel.control.withdrawn_b = true;
    }

    // 4. Return rent to the creator account (if all funds are withdrawn).
    if channel.is_withdrawn() {
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

        // 6. Emit withdraw event.
        msg!(
            "Event: perun:pay_c channel_id: {:?}, party_idx: {}, all_withdrawn: true",
            channel_id,
            party_idx,
        );
    } else {
        // Serialize the updated channel state back to the account.
        channel.serialize(&mut &mut channel_account.try_borrow_mut_data()?[..])?;

        // 6. Emit withdraw event.
        msg!(
            "Event: perun:withdraw channel_id: {:?}, party_idx: {}",
            channel_id,
            party_idx,
        );
    }

    Ok(())
}
