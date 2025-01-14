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

use std::cmp::Ordering;

use bp::dbc::opret::OpretProof;
use bp::dbc::tapret::TapretProof;
use bp::dbc::Anchor;
use bp::Txid;
use commit_verify::mpc;
use strict_encoding::StrictDumb;

use crate::{BundleId, ContractId, TransitionBundle, WitnessId, WitnessOrd, XChain, LIB_NAME_RGB};

#[derive(Clone, Eq, PartialEq, Debug)]
#[derive(StrictType, StrictDumb, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
pub struct AnchoredBundle {
    pub anchor: XAnchor,
    pub bundle: TransitionBundle,
}

impl AnchoredBundle {
    #[inline]
    pub fn bundle_id(&self) -> BundleId { self.bundle.bundle_id() }
}

pub type XAnchor<P = mpc::MerkleProof> = XChain<AnchorSet<P>>;

impl<P: mpc::Proof + StrictDumb> XAnchor<P> {
    #[inline]
    pub fn witness_id(&self) -> Option<WitnessId> { self.maybe_map_ref(|set| set.txid()) }

    #[inline]
    pub fn witness_id_unchecked(&self) -> WitnessId { self.map_ref(|set| set.txid_unchecked()) }
}

impl XAnchor<mpc::MerkleBlock> {
    pub fn known_bundle_ids(&self) -> impl Iterator<Item = (BundleId, ContractId)> + '_ {
        match self {
            XAnchor::Bitcoin(anchor) | XAnchor::Liquid(anchor) => anchor.known_bundle_ids(),
        }
    }

    pub fn to_merkle_proof(
        &self,
        contract_id: ContractId,
    ) -> Result<XAnchor<mpc::MerkleProof>, mpc::LeafNotKnown> {
        self.clone().into_merkle_proof(contract_id)
    }

    pub fn into_merkle_proof(
        self,
        contract_id: ContractId,
    ) -> Result<XAnchor<mpc::MerkleProof>, mpc::LeafNotKnown> {
        self.try_map(|a| a.into_merkle_proof(contract_id))
    }
}

impl XAnchor<mpc::MerkleProof> {
    pub fn to_merkle_block(
        &self,
        contract_id: ContractId,
        bundle_id: BundleId,
    ) -> Result<XAnchor<mpc::MerkleBlock>, mpc::InvalidProof> {
        self.clone().into_merkle_block(contract_id, bundle_id)
    }

    pub fn into_merkle_block(
        self,
        contract_id: ContractId,
        bundle_id: BundleId,
    ) -> Result<XAnchor<mpc::MerkleBlock>, mpc::InvalidProof> {
        self.try_map(|a| a.into_merkle_block(contract_id, bundle_id))
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
#[derive(StrictType, StrictDumb, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB, tags = custom, dumb = Self::Tapret(strict_dumb!()))]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
pub enum AnchorSet<P: mpc::Proof + StrictDumb = mpc::MerkleProof> {
    #[strict_type(tag = 0x01)]
    Tapret(Anchor<P, TapretProof>),
    #[strict_type(tag = 0x02)]
    Opret(Anchor<P, OpretProof>),
    #[strict_type(tag = 0x03)]
    Dual {
        tapret: Anchor<P, TapretProof>,
        opret: Anchor<P, OpretProof>,
    },
}

impl<P: mpc::Proof + StrictDumb> AnchorSet<P> {
    pub fn txid(&self) -> Option<Txid> {
        match self {
            AnchorSet::Tapret(a) => Some(a.txid),
            AnchorSet::Opret(a) => Some(a.txid),
            AnchorSet::Dual { tapret, opret } if tapret.txid == opret.txid => Some(tapret.txid),
            _ => None,
        }
    }

    pub fn txid_unchecked(&self) -> Txid {
        match self {
            AnchorSet::Tapret(a) => a.txid,
            AnchorSet::Opret(a) => a.txid,
            AnchorSet::Dual { tapret, opret: _ } => tapret.txid,
        }
    }

    pub fn from_split(
        tapret: Option<Anchor<P, TapretProof>>,
        opret: Option<Anchor<P, OpretProof>>,
    ) -> Option<Self> {
        Some(match (tapret, opret) {
            (Some(tapret), Some(opret)) => Self::Dual { tapret, opret },
            (Some(tapret), None) => Self::Tapret(tapret),
            (None, Some(opret)) => Self::Opret(opret),
            (None, None) => return None,
        })
    }

    #[allow(clippy::type_complexity)]
    pub fn as_split(&self) -> (Option<&Anchor<P, TapretProof>>, Option<&Anchor<P, OpretProof>>) {
        match self {
            AnchorSet::Tapret(tapret) => (Some(tapret), None),
            AnchorSet::Opret(opret) => (None, Some(opret)),
            AnchorSet::Dual { tapret, opret } => (Some(tapret), Some(opret)),
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn into_split(self) -> (Option<Anchor<P, TapretProof>>, Option<Anchor<P, OpretProof>>) {
        match self {
            AnchorSet::Tapret(tapret) => (Some(tapret), None),
            AnchorSet::Opret(opret) => (None, Some(opret)),
            AnchorSet::Dual { tapret, opret } => (Some(tapret), Some(opret)),
        }
    }

    pub fn mpc_proofs(&self) -> impl Iterator<Item = &P> {
        let (t, o) = self.as_split();
        t.map(|a| &a.mpc_proof)
            .into_iter()
            .chain(o.map(|a| &a.mpc_proof))
    }
}

impl AnchorSet<mpc::MerkleProof> {
    pub fn to_merkle_block(
        &self,
        contract_id: ContractId,
        bundle_id: BundleId,
    ) -> Result<AnchorSet<mpc::MerkleBlock>, mpc::InvalidProof> {
        self.clone().into_merkle_block(contract_id, bundle_id)
    }

    pub fn into_merkle_block(
        self,
        contract_id: ContractId,
        bundle_id: BundleId,
    ) -> Result<AnchorSet<mpc::MerkleBlock>, mpc::InvalidProof> {
        let (tapret, opret) = self.into_split();
        let tapret = tapret
            .map(|t| t.into_merkle_block(contract_id, bundle_id))
            .transpose()?;
        let opret = opret
            .map(|o| o.into_merkle_block(contract_id, bundle_id))
            .transpose()?;
        Ok(AnchorSet::from_split(tapret, opret).expect("one must be non-None"))
    }
}

impl AnchorSet<mpc::MerkleBlock> {
    pub fn known_bundle_ids(&self) -> impl Iterator<Item = (BundleId, ContractId)> + '_ {
        self.mpc_proofs().flat_map(|p| {
            p.to_known_message_map()
                .into_iter()
                .map(|(p, m)| (m.into(), p.into()))
        })
    }

    pub fn to_merkle_proof(
        &self,
        contract_id: ContractId,
    ) -> Result<AnchorSet<mpc::MerkleProof>, mpc::LeafNotKnown> {
        self.clone().into_merkle_proof(contract_id)
    }

    pub fn into_merkle_proof(
        self,
        contract_id: ContractId,
    ) -> Result<AnchorSet<mpc::MerkleProof>, mpc::LeafNotKnown> {
        let (tapret, opret) = self.into_split();
        let tapret = tapret
            .map(|t| t.into_merkle_proof(contract_id))
            .transpose()?;
        let opret = opret
            .map(|o| o.into_merkle_proof(contract_id))
            .transpose()?;
        Ok(AnchorSet::from_split(tapret, opret).expect("one must be non-None"))
    }
}

/// Txid and height information ordered according to the RGB consensus rules.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display)]
#[derive(StrictType, StrictDumb, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
#[display("{witness_id}/{witness_ord}")]
pub struct WitnessAnchor {
    pub witness_ord: WitnessOrd,
    pub witness_id: WitnessId,
}

impl PartialOrd for WitnessAnchor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for WitnessAnchor {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            return Ordering::Equal;
        }
        match self.witness_ord.cmp(&other.witness_ord) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => self.witness_id.cmp(&other.witness_id),
        }
    }
}

impl WitnessAnchor {
    pub fn from_mempool(witness_id: WitnessId) -> Self {
        WitnessAnchor {
            witness_ord: WitnessOrd::OffChain,
            witness_id,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[display(lowercase)]
#[derive(StrictType, StrictDumb, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_RGB, tags = repr, into_u8, try_from_u8)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate", rename_all = "camelCase")
)]
#[repr(u8)]
pub enum Layer1 {
    #[strict_type(dumb)]
    Bitcoin = 0,
    Liquid = 1,
}
