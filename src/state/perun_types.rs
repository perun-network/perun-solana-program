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

use std::fmt::Display;

use crate::{
    error::PerunError,
    state::{
        multi::{convert_cross_assets, Chain},
        sol::{AllocationSol, StateSol},
        CrossAsset,
    },
};
use alloy_primitives::{
    keccak256, Address as EthAddress, Bytes as PrimBytes, FixedBytes, Uint, U256,
};
use alloy_sol_types::SolValue;

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
    pub l2_pubkey: [u8; 65], // Uncompressed secp256k1 public key
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
    pub bal_a: Vec<u64>,
    pub bal_b: Vec<u64>,
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
    pub fn convert_allocation(&self) -> Result<AllocationSol, PerunError> {
        // Ensure that there are exactly two cross-chain assets
        let cross_assets = self.balances.tokens.clone();
        // Determine backends based on the address types in cross_assets
        let backends: [U256; 2] = [
            {
                // Check if the chain is 6
                if unsafe { cross_assets.get_unchecked(0) }.chain
                    != Chain::new(Chain::SOLANA_BACKEND_ID)
                {
                    U256::from(1) // Ethereum
                } else {
                    U256::from(Chain::SOLANA_BACKEND_ID) // Solana
                }
            },
            {
                // Check if the chain is 6
                if unsafe { cross_assets.get_unchecked(1).chain }
                    != Chain::new(Chain::SOLANA_BACKEND_ID)
                {
                    U256::from(1) // Ethereum
                } else {
                    U256::from(Chain::SOLANA_BACKEND_ID) // Solana
                }
            },
        ];

        // Use convert_cross_assets to convert both assets
        let (asset_sol_0, asset_sol_1) = convert_cross_assets(&cross_assets)?;

        // Convert balances
        let bals_cc_a = U256::from(
            *self
                .balances
                .bal_a
                .get(0)
                .ok_or(PerunError::ConversionError)? as u128,
        );
        let bals_solana_a = U256::from(
            *self
                .balances
                .bal_a
                .get(1)
                .ok_or(PerunError::ConversionError)? as u128,
        );
        let bals_cc_b = U256::from(
            *self
                .balances
                .bal_b
                .get(0)
                .ok_or(PerunError::ConversionError)? as u128,
        );
        let bals_solana_b = U256::from(
            *self
                .balances
                .bal_b
                .get(1)
                .ok_or(PerunError::ConversionError)? as u128,
        );

        // Construct the AllocationSol with the vectors
        Ok(AllocationSol {
            assets: [asset_sol_0, asset_sol_1].to_vec(), // Directly convert to a vector
            backends: backends.to_vec(),
            balances: [
                [bals_cc_a, bals_cc_b].to_vec(),
                [bals_solana_a, bals_solana_b].to_vec(),
            ]
            .to_vec(),
            locked: [].to_vec(),
        })
    }

    pub fn convert_state(&self) -> Result<StateSol, PerunError> {
        // let channel_id_xdr = state.channel_id.clone().to_xdr(e);

        let channel_id = self.channel_id.clone();

        // Define the expected length
        let chanid_len = 32;

        // Check if the length of channel_id_xdr matches the expected length
        if channel_id.len() != chanid_len {
            return Err(PerunError::InvalidChanIdSize); // Ensure this error variant is defined
        }

        let channel_id_alloy = FixedBytes::from_slice(&channel_id);
        let app_data_alloy = PrimBytes::copy_from_slice(&[]);
        let is_final_alloy = self.finalized;

        let outcome = self.convert_allocation()?;

        Ok(StateSol {
            channelID: channel_id_alloy,
            version: self.version,
            outcome,
            appData: app_data_alloy,
            isFinal: is_final_alloy,
        })
    }

    pub fn hash_state_eth_prefixed(&self) -> Result<FixedBytes<32>, PerunError> {
        let state_sik = self.convert_state()?;
        let state_abienc = state_sik.abi_encode();

        let state_sol_hashed = keccak256(&state_abienc);

        let prefix = b"\x19Ethereum Signed Message:\n32";
        let prefix_hash = [prefix.as_ref(), &state_sol_hashed[..]].concat();

        let state_sol_prefix_hash = keccak256(&prefix_hash);
        Ok(state_sol_prefix_hash)
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
    /// creator of the channel, i.e. the participant that created it.
    pub creator: Pubkey,
}
impl Control {
    pub const SPACE: usize = 46; // 1 + 1 + 1 + 1 + 1 + 1 + 8 + 32
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

    /// is_funded checks if both participants have funded the channel.
    pub fn is_funded(&self) -> bool {
        self.control.funded_a && self.control.funded_b
    }

    // is_withdrawn checks if both participants have withdrawn their balances from a closed channel.
    pub fn is_withdrawn(&self) -> bool {
        self.control.closed && self.control.withdrawn_a && self.control.withdrawn_b
    }
}
