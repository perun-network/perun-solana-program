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
        state::perun_types::{Channel, ChannelID},
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::{clock::Clock, Sysvar},
    },
};

/// process_force_close forcibly closes a channel after it has been disputed at least once and the
/// relative timelock (challenge_duration) has elapsed since the latest dispute.
pub fn process_force_close(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    channel_id: ChannelID,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let channel_account = next_account_info(account_info_iter)?;

    // 1. Get the channel PDA.
    let (channel_pda, _bump) = Pubkey::find_program_address(
        &[Channel::SEED_PREFIX.as_bytes(), channel_id.as_bytes()],
        program_id,
    );

    if channel_account.key != &channel_pda {
        msg!("Wrong PDA passed");
        return Err(ProgramError::InvalidArgument);
    }

    // 2. Deserialize and validate.
    let channel = &mut Channel::try_from_slice(&channel_account.try_borrow_mut_data()?)?;
    if channel.control.closed {
        msg!("Channel is already closed");
        return Err(PerunError::ForceCloseOnClosedChannel.into());
    }
    if !channel.is_funded() {
        msg!("Channel is not funded");
        return Err(PerunError::OperationOnUnfundedChannel.into());
    }
    // The channel must have been disputed on-chain at least once.
    if !channel.control.disputed {
        msg!("Channel must be disputed before force closing");
        return Err(PerunError::ForceCloseOnUndisputedChannel.into());
    }
    // Verify that the time-lock has expired.
    if !is_timelock_expired(channel) {
        msg!("Time-lock has not expired yet");
        return Err(PerunError::TimelockNotExpired.into());
    }

    // 3. Set the channel to closed.
    channel.control.closed = true;
    channel.serialize(&mut &mut channel_account.try_borrow_mut_data()?[..])?;

    // 4. Emit the ForceClose event.
    msg!("Event: perun:f_close channel_id: {:?}", channel_id,);
    Ok(())
}

/// is_timelock_expired checks, if the relative timelock that (re)starts with a dispute
/// has expired. The timelock is considered to be expired, iff the channel has been disputed
/// at least once and the last dispute was at least challenge_duration seconds ago.
pub fn is_timelock_expired(channel: &Channel) -> bool {
    if !channel.control.disputed {
        return false;
    }
    let clock = Clock::get().unwrap();
    let current_time = clock.unix_timestamp as u64;
    msg!(
        "Current time: {}, Channel control timestamp: {}, Challenge duration: {}",
        current_time,
        channel.control.timestamp,
        channel.params.challenge_duration
    );
    channel.control.timestamp + channel.params.challenge_duration <= current_time
}
