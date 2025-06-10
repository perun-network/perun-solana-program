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
//! Error types
use {
    num_derive::FromPrimitive,
    solana_program::{msg, program_error::ProgramError},
    thiserror::Error,
};
/// Errors that may be returned by the Perun program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq, PartialOrd, Ord)]
pub enum PerunError {
    #[error("Invalid instruction")]
    InvalidInstruction = 0,

    #[error("Channel ID mismatch")]
    ChannelIDMismatch,

    #[error("Invalid version number")]
    InvalidVersionNumber,

    #[error("Channel is alreay final when opened")]
    OpenOnFinalState,

    #[error("Channel already exists")]
    ChannelAlreadyExists,

    #[error("Channel not found")]
    ChannelNotFound,

    #[error("Encoding error")]
    EncodingError,

    #[error("Invalid actor")]
    InvalidActor,

    #[error("Channel is already funded")]
    AlreadyFunded,

    #[error("Cannot close channel on non-final state")]
    CloseOnNonFinalState,

    #[error("Secp256k1 recovery failed")]
    SecpRecoveryFailed,

    #[error("Malformed verification input")]
    MalformedVerificationInput,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Operation not allowed on unfunded channel")]
    OperationOnUnfundedChannel,

    #[error("Withdraw not allowed on open channel")]
    WithdrawOnOpenChannel,

    #[error("Fund is already withdrawn")]
    AlreadyWithdrawn,

    #[error("Dispute not allowed on closed channel")]
    DisputeOnClosedChannel,

    #[error("Invalid state transition")]
    InvalidStateTransition,

    #[error("Force close not allowed on closed channel")]
    ForceCloseOnClosedChannel,

    #[error("Force close not allowed on undisputed channel")]
    ForceCloseOnUndisputedChannel,

    #[error("Timelock not expired")]
    TimelockNotExpired,

    #[error("Funding aborted due to channel state")]
    AbortFundingOnFundedChannel,

    #[error("Funding aborted due to channel being closed")]
    AbortFundingOnClosedChannel,

    #[error("Funding aborted due to channel being disputed")]
    AbortFundingOnDisputedChannel,

    #[error("Funding aborted due to insufficient funds")]
    AbortFundingWithoutFunds,

    #[error("Verification failed")]
    VerificationFailed,

    #[error("Invalid public key type")]
    InvalidPubKeyType,

    #[error("Invalid key type")]
    InvalidKeyType,

    #[error("Conversion error")]
    ConversionError,

    #[error("Wrong asset type")]
    WrongAssetType,

    #[error("Invalid channel ID size")]
    InvalidChanIdSize,

    #[error("Wrong channel type")]
    WrongChannelTypeErr,

    #[error("Invalid address type")]
    InvalidAddressType,
}

impl From<PerunError> for ProgramError {
    fn from(e: PerunError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
