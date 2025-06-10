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

use alloy::rpc::types::state;
use borsh::{BorshDeserialize, BorshSerialize};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

use solana_program::pubkey::Pubkey;

use crate::{error::PerunError, state::sol::AssetSol};

use alloy_primitives::{
    Address as EthAddress, Bytes as PrimBytes, FixedBytes, U256, Uint, keccak256,
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
        let r_s = &sig[0..64];
        let v = sig[64];
        if v > 1 {
            return Err(PerunError::InvalidSignature.into());
        }

        let mut state_sol_abi: [u8; 32] = [0u8; 32];
        let ssl = msg_bytes.as_slice();
        state_sol_abi.copy_from_slice(&ssl);

        // 2. Convert to Signature and RecoveryId
        let signature =
            Signature::from_slice(r_s).map_err(|_| PerunError::MalformedVerificationInput)?;
        let recovery_id =
            RecoveryId::try_from(v).map_err(|_| PerunError::MalformedVerificationInput)?;

        // 3. Recover public key
        let recovered_key = VerifyingKey::recover_from_msg(&state_sol_abi, &signature, recovery_id)
            .map_err(|_| PerunError::SecpRecoveryFailed)?;

        // 4. Convert recovered key to uncompressed SEC1 format
        let recovered_bytes = recovered_key.to_encoded_point(false); // false = uncompressed
        let recovered_pubkey = recovered_bytes.as_bytes();

        if recovered_pubkey.len() != 65 {
            return Err(PerunError::SecpRecoveryFailed.into());
        }

        // 5. Compare recovered key to stored key
        if &self.key == recovered_pubkey {
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
