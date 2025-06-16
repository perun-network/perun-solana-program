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

use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::{pubkey::Pubkey, secp256k1_recover::secp256k1_recover};

use crate::{error::PerunError, state::sol::AssetSol};

use alloy_primitives::{
    keccak256, Address as EthAddress, Bytes as PrimBytes, FixedBytes, Uint, U256,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq, Copy)]
pub struct Chain(u64);
impl Chain {
    pub const SPACE: usize = 8; // u64 size in bytes
    pub const SOLANA_BACKEND_ID: u64 = 6;

    pub fn new(value: u64) -> Self {
        Chain(value)
    }
    pub fn as_u64(&self) -> u64 {
        // Use `u64` here to avoid data loss
        self.0
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct CrossAsset {
    pub chain: Chain,
    pub solana_address: Pubkey,
    pub eth_address: [u8; 20],
}
impl CrossAsset {
    pub const SPACE: usize = Chain::SPACE + 32 + 20; // Chain (u64) + Solana address (Pubkey) + Ethereum address (20 bytes).
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ChannelPubKeyCross {
    pub key: [u8; 65], // Uncompressed secp256k1 public key
}

impl ChannelPubKeyCross {
    pub fn verify_signature_cross(
        &self,
        msg_bytes: FixedBytes<32>, // 32-byte message hash (e.g., keccak256(data))],
        sig: &[u8; 65],            // r || s || v (Ethereum-style)
    ) -> Result<(), PerunError> {
        // 1. Extract r,s (first 64 bytes) and v (last byte)
        let r_s: &[u8; 64] = sig[0..64].try_into().unwrap();
        let v = sig[64];

        let recovery_id = match v {
            0 | 1 => v,
            27 | 28 => v - 27,
            _ => return Err(PerunError::InvalidSignature),
        };

        let mut state_sol_abi: [u8; 32] = [0u8; 32];
        let ssl = msg_bytes.as_slice();
        state_sol_abi.copy_from_slice(&ssl);

        // 3. Recover public key
        let recovered_pub_key = secp256k1_recover(&state_sol_abi[..], recovery_id, r_s)
            .map_err(|_| PerunError::SecpRecoveryFailed)?;
        // Compare to stored key, skipping the 0x04 prefix (first byte)
        let expected_key = &self.key[1..]; // [X || Y], 64 bytes

        // 4. Compare recovered key to stored key
        if recovered_pub_key.to_bytes() == expected_key {
            Ok(())
        } else {
            Err(PerunError::InvalidSignature.into())
        }
    }
}

pub fn convert_cross_assets(
    cross_assets: &Vec<CrossAsset>,
) -> Result<(AssetSol, AssetSol), PerunError> {
    if cross_assets.len() != 2 {
        return Err(PerunError::ConversionError.into());
    }

    let convert_asset = |cross_asset: &CrossAsset| -> Result<AssetSol, PerunError> {
        let chain_id = U256::from(cross_asset.chain.as_u64());

        let zero_eth_address = EthAddress::from_slice(&[0u8; 20]);
        let zero_cc_address = vec![0u8; 32];

        let (eth_holder, cc_holder) = if chain_id != U256::from(Chain::SOLANA_BACKEND_ID) {
            // Ethereum side
            let eth_holder = EthAddress::from_slice(cross_asset.eth_address.clone().as_ref());
            (eth_holder, zero_cc_address)
        } else {
            // Solana side
            let cc_holder = cross_asset.solana_address.clone(); // Should already be 32 bytes
            (zero_eth_address, cc_holder.as_array().to_vec())
        };

        Ok(AssetSol {
            chainID: chain_id,
            ethHolder: eth_holder,
            ccHolder: cc_holder.into(),
        })
    };

    Ok((
        convert_asset(&cross_assets[0])?,
        convert_asset(&cross_assets[1])?,
    ))
}
