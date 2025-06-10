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
            multi::ChannelPubKeyCross,
            perun_types::{Channel, ChannelState},
        },
    },
    borsh::BorshDeserialize,
    solana_program::{
        account_info::{AccountInfo, next_account_info},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

pub fn process_close(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    state: ChannelState,
    sig_a: [u8; 65],
    sig_b: [u8; 65],
) -> ProgramResult {
    msg!(
        "Processing Close instruction with program_id: {:?}, state: {:?}, sig_a: {:?}, sig_b: {:?}",
        program_id,
        state,
        sig_a,
        sig_b
    );

    let account_info_iter = &mut accounts.iter();
    let channel_account = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // 1. Get the channel PDA.
    let (channel_pda, _bump) = Pubkey::find_program_address(
        &[Channel::SEED_PREFIX.as_bytes(), state.channel_id.as_ref()],
        program_id,
    );

    if channel_account.key != &channel_pda {
        msg!("Wrong PDA passed");
        return Err(ProgramError::InvalidArgument);
    }

    // 2. Deserialize and validate.
    let channel = &mut Channel::try_from_slice(&channel_account.try_borrow_mut_data()?)?;

    if !state.finalized {
        msg!("Channel state is not finalized");
        return Err(PerunError::CloseOnNonFinalState.into());
    }

    if !channel.is_funded() {
        msg!("Channel is not funded");
        return Err(PerunError::OperationOnUnfundedChannel.into());
    }

    // 3. Verify both parties' signatures on the submitted final state.
    let hash = state.hash_state_eth_prefixed()?;

    let pub_key_a = ChannelPubKeyCross {
        key: channel.params.a.l2_pubkey.clone(),
    };
    let pub_key_b = ChannelPubKeyCross {
        key: channel.params.b.l2_pubkey.clone(),
    };
    pub_key_a
        .verify_signature_cross(hash.clone(), &sig_a)
        .map_err(|_| PerunError::InvalidSignature)?;

    pub_key_b
        .verify_signature_cross(hash, &sig_b)
        .map_err(|_| PerunError::InvalidSignature)?;

    // 4. Update channel state to closed.
    channel.control.closed = true;
    channel.state = state.clone();

    // 5. Emit close event.
    msg!(
        "Event: perun:close channel_id: {:#?}: state {:?}",
        state.channel_id,
        state.clone()
    );

    Ok(())
}
