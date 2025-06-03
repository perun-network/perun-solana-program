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

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// Participant represents a participant in the channel.
// All channels have two participants.
pub struct Participant {
    // solana_addr represents the participant's on-chain address.
    // The participant receives payments on this address.
    pub solana_address: solana_pubkey::Pubkey,
    pub cc_address: [u8; 20],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// ChannelID is the unique identifier for a channel.
// It is a 32-byte array that can be derived from the channel's participants and parameters.
pub enum ChannelID {
    ID([u8; 32]),
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// Balances represents the balance distribution in a channel.
pub struct Balances {
    // token represents a channel's asset / currency. Currently this contract
    // supports single-asset channels, but multi-asset support is possible.
    tokens: Vec<CrossAsset>,
    pub bal_a: Vec<i128>,
    pub bal_b: Vec<i128>,
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

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
// ChannelState represents the on-chain state of a channel.
pub struct ChannelState {
    // channel_id is the unique identifier for the channel.
    // It is the hash of the channel's Params.
    pub channel_id: [u8; 32],
    // balances represent the balance distribution in the channel.
    pub balances: Balances,
    /// version is incremented on off-chain state updates and therefore establishes
    /// a strict happened-before relation between all states that belong to a channel.
    pub version: u64,
    /// finalized signals whether a state is considered final. A final state can be closed
    /// gracefully by using the `close` endpoint with both participants' signatures on the state.
    pub finalized: bool,
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

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
/// Channel is the on-chain representation of a channel.
pub struct Channel {
    /// params contains the (constant) channel parameters.
    pub params: Params,
    /// state contains the latest (on-chain) channel state.
    /// Note that there can be off-chain state updates that are more recent (have higher version number)
    /// than the registered on-chain state for a channel.
    pub state: ChannelState,
    /// control contains the channel's control bits.
    pub control: Control,
}
