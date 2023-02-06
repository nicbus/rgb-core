// RGB Core Library: consensus layer for RGB smart contracts.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2019-2023 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2023 LNP/BP Standards Association. All rights reserved.
// Copyright (C) 2019-2023 Dr Maxim Orlovsky. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::vec;

use amplify::confinement::MediumVec;
use commit_verify::merkle::{MerkleLeaves, MerkleNode};
use commit_verify::CommitmentId;

use super::state::{AttachmentPair, DeclarativePair, FungiblePair, StructuredPair};
use super::{seal, AssignedState, StateType, UnknownDataError};
use crate::LIB_NAME_RGB;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[derive(StrictType, StrictDumb, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB, tags = custom, dumb = Self::Declarative(strict_dumb!()))]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "snake_case")
)]
pub enum TypedState {
    // TODO: Consider using non-empty variants
    #[strict_type(tag = 0x00)]
    Declarative(MediumVec<AssignedState<DeclarativePair>>),
    #[strict_type(tag = 0x01)]
    Fungible(MediumVec<AssignedState<FungiblePair>>),
    #[strict_type(tag = 0x02)]
    Structured(MediumVec<AssignedState<StructuredPair>>),
    #[strict_type(tag = 0xFF)]
    Attachment(MediumVec<AssignedState<AttachmentPair>>),
}

impl TypedState {
    pub fn is_empty(&self) -> bool {
        match self {
            TypedState::Declarative(set) => set.is_empty(),
            TypedState::Fungible(set) => set.is_empty(),
            TypedState::Structured(set) => set.is_empty(),
            TypedState::Attachment(set) => set.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            TypedState::Declarative(set) => set.len(),
            TypedState::Fungible(set) => set.len(),
            TypedState::Structured(set) => set.len(),
            TypedState::Attachment(set) => set.len(),
        }
    }

    #[inline]
    pub fn state_type(&self) -> StateType {
        match self {
            TypedState::Declarative(_) => StateType::Void,
            TypedState::Fungible(_) => StateType::Fungible,
            TypedState::Structured(_) => StateType::Structured,
            TypedState::Attachment(_) => StateType::Attachment,
        }
    }

    #[inline]
    pub fn is_declarative(&self) -> bool { matches!(self, TypedState::Declarative(_)) }

    #[inline]
    pub fn is_fungible(&self) -> bool { matches!(self, TypedState::Fungible(_)) }

    #[inline]
    pub fn is_structured(&self) -> bool { matches!(self, TypedState::Structured(_)) }

    #[inline]
    pub fn is_attachment(&self) -> bool { matches!(self, TypedState::Attachment(_)) }

    #[inline]
    pub fn as_declarative(&self) -> &[AssignedState<DeclarativePair>] {
        match self {
            TypedState::Declarative(set) => set,
            _ => Default::default(),
        }
    }

    #[inline]
    pub fn as_fungible(&self) -> &[AssignedState<FungiblePair>] {
        match self {
            TypedState::Fungible(set) => set,
            _ => Default::default(),
        }
    }

    #[inline]
    pub fn as_structured(&self) -> &[AssignedState<StructuredPair>] {
        match self {
            TypedState::Structured(set) => set,
            _ => Default::default(),
        }
    }

    #[inline]
    pub fn as_attachment(&self) -> &[AssignedState<AttachmentPair>] {
        match self {
            TypedState::Attachment(set) => set,
            _ => Default::default(),
        }
    }

    #[inline]
    pub fn as_declarative_mut(&mut self) -> Option<&mut MediumVec<AssignedState<DeclarativePair>>> {
        match self {
            TypedState::Declarative(set) => Some(set),
            _ => None,
        }
    }

    #[inline]
    pub fn as_fungible_mut(&mut self) -> Option<&mut MediumVec<AssignedState<FungiblePair>>> {
        match self {
            TypedState::Fungible(set) => Some(set),
            _ => None,
        }
    }

    #[inline]
    pub fn as_structured_mut(&mut self) -> Option<&mut MediumVec<AssignedState<StructuredPair>>> {
        match self {
            TypedState::Structured(set) => Some(set),
            _ => None,
        }
    }

    #[inline]
    pub fn as_attachment_mut(&mut self) -> Option<&mut MediumVec<AssignedState<AttachmentPair>>> {
        match self {
            TypedState::Attachment(set) => Some(set),
            _ => None,
        }
    }

    /// If seal definition does not exist, returns [`UnknownDataError`]. If the
    /// seal is confidential, returns `Ok(None)`; otherwise returns revealed
    /// seal data packed as `Ok(Some(`[`seal::Revealed`]`))`
    pub fn revealed_seal_at(&self, index: u16) -> Result<Option<seal::Revealed>, UnknownDataError> {
        Ok(match self {
            TypedState::Declarative(vec) => vec
                .get(index as usize)
                .ok_or(UnknownDataError)?
                .revealed_seal(),
            TypedState::Fungible(vec) => vec
                .get(index as usize)
                .ok_or(UnknownDataError)?
                .revealed_seal(),
            TypedState::Structured(vec) => vec
                .get(index as usize)
                .ok_or(UnknownDataError)?
                .revealed_seal(),
            TypedState::Attachment(vec) => vec
                .get(index as usize)
                .ok_or(UnknownDataError)?
                .revealed_seal(),
        })
    }

    pub fn to_confidential_seals(&self) -> Vec<seal::Confidential> {
        match self {
            TypedState::Declarative(s) => s
                .iter()
                .map(AssignedState::<_>::to_confidential_seal)
                .collect(),
            TypedState::Fungible(s) => s
                .iter()
                .map(AssignedState::<_>::to_confidential_seal)
                .collect(),
            TypedState::Structured(s) => s
                .iter()
                .map(AssignedState::<_>::to_confidential_seal)
                .collect(),
            TypedState::Attachment(s) => s
                .iter()
                .map(AssignedState::<_>::to_confidential_seal)
                .collect(),
        }
    }
}

impl MerkleLeaves for TypedState {
    type Leaf = MerkleNode;
    type LeafIter = vec::IntoIter<MerkleNode>;

    fn merkle_leaves(&self) -> Self::LeafIter {
        match self {
            TypedState::Declarative(vec) => vec
                .iter()
                .map(AssignedState::<DeclarativePair>::commitment_id)
                .collect::<Vec<_>>(),
            TypedState::Fungible(vec) => vec
                .iter()
                .map(AssignedState::<FungiblePair>::commitment_id)
                .collect::<Vec<_>>(),
            TypedState::Structured(vec) => vec
                .iter()
                .map(AssignedState::<StructuredPair>::commitment_id)
                .collect::<Vec<_>>(),
            TypedState::Attachment(vec) => vec
                .iter()
                .map(AssignedState::<AttachmentPair>::commitment_id)
                .collect::<Vec<_>>(),
        }
        .into_iter()
    }
}
