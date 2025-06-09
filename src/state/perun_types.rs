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

use crate::state::CrossAsset;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// Participant represents a participant in the channel.
// All channels have two participants.
pub struct Participant {
    // solana_addr represents the participant's on-chain address.
    // The participant receives payments on this address.
    pub solana_address: Pubkey,
    pub cc_address: [u8; 20],
}
impl Participant {
    pub const SPACE: usize = 32 + 20; // 32 bytes for Pubkey + 20 bytes for Ethereum address
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// ChannelID is the unique identifier for a channel.
// It is a 32-byte array that can be derived from the channel's participants and parameters.
pub enum ChannelID {
    ID([u8; 32]),
}
impl ChannelID {
    pub const SPACE: usize = 32; // 32 bytes for the channel ID

    pub fn as_bytes(&self) -> &[u8; 32] {
        match self {
            ChannelID::ID(id) => id,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// Balances represents the balance distribution in a channel.
pub struct Balances {
    // token represents a channel's asset / currency. Currently this contract
    // supports single-asset channels, but multi-asset support is possible.
    pub tokens: Vec<CrossAsset>,
    pub bal_a: Vec<i128>,
    pub bal_b: Vec<i128>,
}
impl Balances {
    pub fn get_size(&self) -> usize {
        let tokens_len = self.tokens.len();
        let size_in_bytes = std::mem::size_of::<usize>();
        size_in_bytes + (tokens_len * (CrossAsset::SPACE + 16 + 16))
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// Params is the on-chain version of go-perun's channel.Params.
pub struct Params {
    // Participant A
    pub a: Participant,
    // Participant B
    pub b: Participant,
    // Nonce ensures that the channel is unique (generated off-chain).
    pub nonce: [u8; 32],
    // challange_duration is a duration in seconds. A channel can be force-closed, if it was disputed
    // and the relative time lock is expired (i.e. the last dispute was at least challenge_duration
    // seconds ago).
    pub challenge_duration: u64,
}
impl Params {
    pub const SPACE: usize = Participant::SPACE * 2 + 32 + 8; // 2 participants + nonce + challenge_duration
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// ChannelState represents the on-chain state of a channel.
pub struct ChannelState {
    // channel_id is the unique identifier for the channel.
    // It is the hash of the channel's Params.
    pub channel_id: [u8; 32], // Space = 256
    // balances represent the balance distribution in the channel.
    pub balances: Balances,
    /// version is incremented on off-chain state updates and therefore establishes
    /// a strict happened-before relation between all states that belong to a channel.
    pub version: u64, // Space = 64
    /// finalized signals whether a state is considered final. A final state can be closed
    /// gracefully by using the `close` endpoint with both participants' signatures on the state.
    pub finalized: bool,
}
impl ChannelState {
    pub fn get_size(&self) -> usize {
        let channel_id_size = 32; // 32 bytes
        let balances_size = self.balances.get_size();
        let version_size = 8; // 8 bytes
        let finalized_size = 1; // 1 byte
        channel_id_size + balances_size + version_size + finalized_size
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
/// Control contains additional information that allows
/// the contract to judge the channel's state.
pub struct Control {
    /// funded_a is true, iff A has funded the channel.
    pub funded_a: bool,
    /// funded_b is true, iff B has funded the channel.
    pub funded_b: bool,
    /// closed indicates that a fully funded channel is closed and can be withdrawn from.
    pub closed: bool,
    /// withdrawn_a is true, iff either A has already withrawn their balance from a closed channel
    /// or A's balance in the closed channel was 0 to begin with.
    pub withdrawn_a: bool,
    /// withdrawn_b is true, iff either B has already withrawn their balance from a closed channel
    /// or B's balance in the closed channel was 0 to begin with.
    pub withdrawn_b: bool,
    /// disputed is true, iff the channel was successfully disputed at least once.
    pub disputed: bool,
    /// timestamp must always contain the unix time in seconds of the last successful dispute.
    /// If the channel has not been successfully disputed, the timestamp value is not significant.
    pub timestamp: u64,
}
impl Control {
    pub const SPACE: usize = 14; // 1 + 1 + 1 + 1 + 1 + 1 + 8
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct Channel {
    /// params contains the (constant) channel parameters.
    pub params: Params, //
    /// state contains the latest (on-chain) channel state.
    /// Note that there can be off-chain state updates that are more recent (have higher version number)
    /// than the registered on-chain state for a channel.
    pub state: ChannelState, //
    /// control contains the channel's control bits.
    pub control: Control, // Space = 14
}

impl Channel {
    pub const SEED_PREFIX: &'static str = "channel";

    pub fn get_size(&self) -> usize {
        let params_size = Params::SPACE;
        let state_size = self.state.get_size();
        let control_size = Control::SPACE;
        params_size + state_size + control_size
    }

    pub fn is_funded(&self) -> bool {
        self.control.funded_a && self.control.funded_b
    }
}
