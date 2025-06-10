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
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{AccountInfo, next_account_info},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::{Sysvar, clock::Clock},
    },
};

/// process_dispute processes a dispute instruction for a channel.
/// It registers a given signed state on-chain. Honest parties only need to call dispute,
/// if their peer behaves maliciously or does not respond / crashes.
pub fn process_dispute(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_state: ChannelState,
    sig_a: [u8; 65],
    sig_b: [u8; 65],
) -> ProgramResult {
    msg!(
        "Processing Dispute instruction with program_id: {:?}, state: {:?}, sig_a: {:?}, sig_b: {:?}",
        program_id,
        new_state,
        sig_a,
        sig_b
    );

    let account_info_iter = &mut accounts.iter();
    let channel_account = next_account_info(account_info_iter)?;

    // 1. Get the channel PDA.
    let (channel_pda, _bump) = Pubkey::find_program_address(
        &[
            Channel::SEED_PREFIX.as_bytes(),
            new_state.channel_id.as_ref(),
        ],
        program_id,
    );

    if channel_account.key != &channel_pda {
        msg!("Wrong PDA passed");
        return Err(ProgramError::InvalidArgument);
    }

    // 2. Deserialize and validate.
    let channel = &mut Channel::try_from_slice(&channel_account.try_borrow_mut_data()?)?;
    if !channel.is_funded() {
        msg!("Channel is not funded");
        return Err(PerunError::OperationOnUnfundedChannel.into());
    }
    if channel.control.closed {
        msg!("Channel is already closed");
        return Err(PerunError::DisputeOnClosedChannel.into());
    }
    // Validate the state transition.
    if !is_valid_state_transition(&channel.state, &new_state) {
        msg!("Invalid state transition");
        return Err(PerunError::InvalidStateTransition.into());
    }
    // Verify that the new state is signed by both parties.
    let state_sol_prefix_hash = new_state
        .hash_state_eth_prefixed()
        .expect("hashing state eth style failed");
    let pub_key_a = ChannelPubKeyCross {
        key: channel.params.a.l2_pubkey.clone(),
    };
    let pub_key_b = ChannelPubKeyCross {
        key: channel.params.b.l2_pubkey.clone(),
    };
    pub_key_a
        .verify_signature_cross(state_sol_prefix_hash.clone(), &sig_a)
        .map_err(|_| PerunError::InvalidSignature)?;

    pub_key_b
        .verify_signature_cross(state_sol_prefix_hash.clone(), &sig_b)
        .map_err(|_| PerunError::InvalidSignature)?;

    // 3. Update channel state to disputed.
    channel.control.disputed = true;
    channel.control.timestamp = Clock::get()?.unix_timestamp as u64;
    channel.state = new_state.clone();
    channel.serialize(&mut &mut channel_account.try_borrow_mut_data()?[..])?;

    // 4. Emit the Dispute event.
    msg!(
        "Event: perun:dispute channel_id: {:?}, state: {:?}",
        new_state.channel_id,
        new_state
    );

    Ok(())
}

/// is_valid_state_transition returns true, iff there is a valid transition from the old state
/// to the new state.
pub fn is_valid_state_transition(old: &ChannelState, new: &ChannelState) -> bool {
    // If the old state is final, there can be no "new" state.
    if old.finalized {
        return false;
    }
    // We allow the transition old state = new state, iff their version number is 0, because
    // we want to allow force-closing, if e.g. a party never responds with state updates after
    // opening and funding a channel. To allow a force close the state must be disputed first.
    if old.version == 0 && new.version == 0 {
        return old.eq(&new);
    } else if old.version >= new.version {
        // Aside from the edge-case above, the version must
        // strictly increase.
        return false;
    }
    // The state transition is only valid if they share the same channel id.
    if old.channel_id != new.channel_id {
        return false;
    }
    // Both states must have "coherent balances". That means they must:
    // a) share the same token as asset / currency
    if old.balances.tokens != new.balances.tokens {
        return false;
    }
    // // b) The sum of the balances must be equal.
    for i in 0..old.balances.bal_a.len() {
        if (old.balances.bal_a.get(i).unwrap() + old.balances.bal_b.get(i).unwrap())
            != new.balances.bal_a.get(i).unwrap() + new.balances.bal_b.get(i).unwrap()
        {
            return false;
        }
    }
    return true;
}
