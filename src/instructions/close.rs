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
        account_info::{next_account_info, AccountInfo},
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
    let account_info_iter = &mut accounts.iter();
    let channel_account = next_account_info(account_info_iter)?;

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
    {
        let data = channel_account.try_borrow_data()?;
        let channel = Channel::try_from_slice(&data)?;

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
            key: channel.params.a.l2_pubkey,
        };
        let pub_key_b = ChannelPubKeyCross {
            key: channel.params.b.l2_pubkey,
        };

        pub_key_a
            .verify_signature_cross(&hash, &sig_a)
            .map_err(|_| PerunError::InvalidSignature)?;

        pub_key_b
            .verify_signature_cross(&hash, &sig_b)
            .map_err(|_| PerunError::InvalidSignature)?;
    }
    msg!("Signatures verified successfully");

    // 4. Update channel state to closed.
    let mut data_mut = channel_account.try_borrow_mut_data()?;
    let channel_mut = Channel::try_from_slice(&data_mut)?;

    // Update channel state
    let mut updated_channel = channel_mut;
    updated_channel.control.closed = true;
    updated_channel.state.version = state.version;
    updated_channel.state.balances.bal_a = state.balances.bal_a;
    updated_channel.state.balances.bal_b = state.balances.bal_b;
    updated_channel.state.finalized = state.finalized;
    updated_channel.serialize(&mut &mut data_mut[..])?;

    // 5. Emit close event.
    msg!("Event: perun:close channel_id: {:?}", state.channel_id,);

    Ok(())
}
