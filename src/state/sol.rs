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
//! Solidity <-> Solana data types.

use crate::state::{ChannelID, Params, Participant};
use {
    alloy_primitives::{Address, Bytes as PrimBytes, U256, keccak256},
    alloy_sol_types::SolValue,
    alloy_sol_types::sol,
};

sol! {
    struct ParticipantSol {
        address ethAddress;
        bytes ccAddress;
    }

    struct ParamsSol {
        uint256 challengeDuration;
        uint256 nonce;
        ParticipantSol[] participants;
        address app;
        bool ledgerChannel;
        bool virtualChannel;
    }

    #[derive(Debug)]
    struct StateSol {
        bytes32 channelID;
        uint64 version;
        AllocationSol outcome;
        bytes appData;
        bool isFinal;
    }

    #[derive(Debug)]
    struct AssetSol {
        uint256 chainID;
        address ethHolder;
        bytes ccHolder;
    }

    #[derive(Debug)]
    struct AllocationSol {
        AssetSol[] assets;
        uint256[] backends;
        // Outer dimension are assets, inner dimension are the participants.
        uint256[][] balances;
        SubAllocSol[] locked;
    }

    #[derive(Debug)]
    struct SubAllocSol {
        // ID is the channelID of the subchannel
        bytes32[] ID; // solhint-disable-line var-name-mixedcase
        // balances holds the total balance of the subchannel of every asset.
        uint256[] balances;
        // indexMap maps each sub-channel participant to a parent channel
        // participant such that subPart[i] == parentPart[indexMap[i]].
        uint16[] indexMap;
    }

}

// convert_participant converts a Participant into a ParticipantSol
pub fn convert_participant(participant: &Participant) -> ParticipantSol {
    let solana_addr_array = participant.solana_address.as_ref();
    let cc_addr = participant.cc_address.clone();
    let l2_pubkey = participant.l2_pubkey.clone();

    let cc_addr_alloy = Address::from_slice(&cc_addr);
    let mut part_bytes = [0u8; 117]; // 32 + 20 + 65
    let mut solana_addr_slice = [0u8; 32];
    solana_addr_slice.copy_from_slice(&solana_addr_array);

    part_bytes[0..32].copy_from_slice(&solana_addr_slice); // Solana pubkey
    part_bytes[32..52].copy_from_slice(&cc_addr); // Cross-chain address
    part_bytes[52..117].copy_from_slice(&l2_pubkey); // L2 pubkey

    let solana_addr_alloy = PrimBytes::copy_from_slice(&part_bytes);
    return ParticipantSol {
        ethAddress: cc_addr_alloy,
        ccAddress: solana_addr_alloy,
    };
}

// Function to convert Params to ParamsSol
pub fn convert_params(params: &Params) -> ParamsSol {
    // Convert participants
    let part_sol_a = convert_participant(&params.a);
    let part_sol_b = convert_participant(&params.b);
    let participants_sol = [part_sol_a, part_sol_b].to_vec();

    let nonce_alloy = U256::from_be_bytes(params.nonce);
    // Challenge duration as U256
    let chall_duration = U256::from(params.challenge_duration);

    // Default app address (assuming no app is used)
    let app_alloy = Address::from_slice(&[0u8; 20]); // Null address

    ParamsSol {
        challengeDuration: chall_duration,
        nonce: nonce_alloy,
        participants: participants_sol,
        app: app_alloy,
        ledgerChannel: true,
        virtualChannel: false,
    }
}

pub fn get_channel_id_cross(params: &Params) -> ChannelID {
    let params_sol = convert_params(params);
    let encoded_data = params_sol.abi_encode();

    let hash = keccak256(&encoded_data);
    ChannelID::ID(hash.into())
}
