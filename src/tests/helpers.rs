// Copyright 2025 - See NOTICE file for copyright holders.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use {
    crate::state::{
        ethsig::{EthHash, EthSigner},
        ChannelState,
    },
    alloy_primitives::keccak256,
    alloy_sol_types::SolValue,
    k256::ecdsa::{SigningKey, VerifyingKey},
    rand::thread_rng,
    solana_sdk::signer::keypair::Keypair,
};

pub struct TestKeyPair {
    pub eth_signer: EthSigner, // Only the Ethereum signer
    pub solana_signer: Keypair,
}

pub fn generate_secp_keypair() -> (SigningKey, VerifyingKey) {
    let privkey = SigningKey::random(&mut thread_rng());
    let pubkey = VerifyingKey::from(&privkey);
    (privkey, pubkey)
}

pub fn get_pubkey_secp_bytes(pubkey: &VerifyingKey) -> [u8; 65] {
    let pubkey_bytes: [u8; 65] = pubkey
        .to_encoded_point(false)
        .as_bytes()
        .try_into()
        .unwrap();
    pubkey_bytes
}

pub fn sign_cross_abi(signer: &TestKeyPair, payload: &ChannelState) -> [u8; 65] {
    let state_sol = payload.convert_state().expect("Failed to convert state");
    let state_sol_abi = state_sol.abi_encode();

    let state_sol_hashed = keccak256(&state_sol_abi);
    let state_sol_bytes: [u8; 32] = state_sol_hashed.into();

    let ethhash = EthHash(state_sol_bytes.into());

    let sig1 = signer.eth_signer.sign_eth(&ethhash);
    let sig1_ethbytes = sig1.0;
    sig1_ethbytes
}
