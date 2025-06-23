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
use crate::state::perun_types::{ChannelID, ChannelState, Params};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum PerunInstruction {
    Open {
        params: Params,
        state: ChannelState,
    },
    Fund {
        channel_id: ChannelID,
        party_idx: bool,
    },
    Close {
        state: ChannelState,
        sig_a: [u8; 65],
        sig_b: [u8; 65],
    },
    ForceClose {
        channel_id: ChannelID,
    },
    Dispute {
        state: ChannelState,
        sig_a: [u8; 65],
        sig_b: [u8; 65],
    },
    Withdraw {
        channel_id: ChannelID,
        party_idx: bool,
        one_withdrawer: bool,
    },
    AbortFunding {
        channel_id: ChannelID,
    },
}

/// Checks if the given payer is a participant in the channel defined by the parameters.
pub fn check_participant(payer: &AccountInfo, params: &Params) -> bool {
    if params.a.solana_address == *payer.key || params.b.solana_address == *payer.key {
        return true;
    }
    false
}
