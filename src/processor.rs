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
//! Program state processor

use crate::{
    error::PerunError,
    instructions::{
        abort_funding, close, dispute, force_close, fund, open,
        perun_instructions::PerunInstruction, withdraw,
    },
};

use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

/// Program state handler.
pub struct Processor {}
impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = PerunInstruction::try_from_slice(instruction_data).map_err(|_| {
            msg!("Invalid Instruction: {:?}", instruction_data);
            PerunError::InvalidInstruction
        })?;

        match instruction {
            PerunInstruction::Open { params, state } => {
                msg!("OPEN");
                open::process_open(program_id, accounts, params, state)
            }
            PerunInstruction::Fund {
                channel_id,
                party_idx,
            } => {
                msg!("FUND");
                fund::process_fund(program_id, accounts, channel_id, party_idx)
            }
            PerunInstruction::Close {
                state,
                sig_a,
                sig_b,
            } => {
                msg!("CLOSE");
                close::process_close(program_id, accounts, state, sig_a, sig_b)
            }
            PerunInstruction::ForceClose { channel_id } => {
                msg!("FORCE_CLOSE");
                force_close::process_force_close(program_id, accounts, channel_id)
            }
            PerunInstruction::Dispute {
                state,
                sig_a,
                sig_b,
            } => {
                msg!("DISPUTE");
                dispute::process_dispute(program_id, accounts, state, sig_a, sig_b)
            }
            PerunInstruction::Withdraw {
                channel_id,
                party_idx,
                one_withdrawer,
            } => {
                msg!("WITHDRAW");
                withdraw::process_withdraw(
                    program_id,
                    accounts,
                    channel_id,
                    party_idx,
                    one_withdrawer,
                )
            }
            PerunInstruction::AbortFunding { channel_id } => {
                msg!("ABORT_FUNDING",);
                abort_funding::process_abort_funding(program_id, accounts, channel_id)
            }
        }
    }
}
