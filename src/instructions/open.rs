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

use crate::{
    error::PerunError,
    state::{
        perun_types::{Channel, ChannelID, ChannelState, Control, Params},
        sol::get_channel_id_cross,
    },
};

use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::{Sysvar, clock::Clock},
};

use borsh::BorshSerialize;

pub fn process_open(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: Params,
    state: ChannelState,
) -> ProgramResult {
    // 1. Compute and validate channel ID
    let cid = get_channel_id_cross(&params);
    if cid != ChannelID::ID(state.channel_id) {
        msg!("ChannelID mismatch");
        return Err(PerunError::ChannelIDMismatch.into());
    }

    let account_info_iter = &mut accounts.iter();
    let channel_account = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let (channel_pda, chanenl_bump) = Pubkey::find_program_address(
        &[Channel::SEED_PREFIX.as_bytes(), state.channel_id.as_ref()],
        program_id,
    );

    if channel_account.key != &channel_pda {
        msg!("Wrong PDA passed");
        return Err(ProgramError::InvalidArgument);
    }

    // 2. Check if channel already exists
    if !channel_account.data_is_empty() {
        msg!("Channel already exists");
        return Err(PerunError::ChannelAlreadyExists.into());
    }

    if state.version != 0 {
        msg!("Invalid initial version number");
        return Err(PerunError::InvalidVersionNumber.into());
    }

    if state.finalized {
        msg!("Cannot open channel on final state");
        return Err(PerunError::OpenOnFinalState.into());
    }

    // 3. Construct control struct
    let clock = Clock::get()?;
    let control = Control {
        funded_a: false,
        funded_b: false,
        closed: false,
        withdrawn_a: false,
        withdrawn_b: false,
        disputed: false,
        timestamp: clock.unix_timestamp as u64,
        creator: payer.key.clone(),
    };

    // 4. Build and serialize the channel
    let channel = Channel {
        params,
        state,
        control,
    };

    let rent = Rent::get()?;
    let channel_span = channel.get_size();
    let required_lamports = rent.minimum_balance(channel_span);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            channel_account.key,
            required_lamports,
            channel_span as u64,
            program_id,
        ),
        &[
            payer.clone(),
            channel_account.clone(),
            system_program.clone(),
        ],
        &[&[
            Channel::SEED_PREFIX.as_bytes(),
            cid.as_bytes(),
            &[chanenl_bump],
        ]],
    )?;
    channel.serialize(&mut &mut channel_account.data.borrow_mut()[..])?;

    // 5. Emit open event.
    msg!(
        "Event: perun::open {:?}: state: {:?}",
        cid,
        channel.state.clone()
    );

    Ok(())
}
