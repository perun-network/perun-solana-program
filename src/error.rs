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
    solana_msg::msg,
    solana_program_error::{ProgramError, ToStr},
    thiserror::Error,
};

/// Errors that may be returned by the Perun program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum Error {}

impl From<Error> for ProgramError {
    fn from(e: Error) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl ToStr for Error {
    fn to_str<E>(&self) -> &'static str
    where
        E: 'static + ToStr + TryFrom<u32>,
    {
        match self {
            _ => {
                msg!("Error: {}", self.to_string());
                "Unknown error"
            }
        }
    }
}
