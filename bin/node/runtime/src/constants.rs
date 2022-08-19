// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
	use node_primitives::Balance;

	pub const MILLICENTS: Balance = 1_000_000_000;
	pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
	pub const DOLLARS: Balance = 100 * CENTS;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
	}
}

#[allow(dead_code)]
/// Time.
pub mod time_dev {
	use node_primitives::{BlockNumber, Moment};

	// These time units are defined in number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	pub const DEFAULT_ASSET_LIFETIME: BlockNumber = MINUTES * 1;
	/// Since BABE is probabilistic this is the average expected block time that
	/// we are targeting. Blocks will be produced at a minimum duration defined
	/// by `SLOT_DURATION`, but some slots will not be allocated to any
	/// authority and hence no block will be produced. We expect to have this
	/// block time on average following the defined slot duration and the value
	/// of `c` configured for BABE (where `1 - c` represents the probability of
	/// a slot being empty).
	/// This value is only used indirectly to define the unit constants below
	/// that are expressed in blocks. The rest of the code should use
	/// `SLOT_DURATION` instead (like the Timestamp pallet for calculating the
	/// minimum period).
	///
	/// If using BABE with secondary slots (default) then all of the slots will
	/// always be assigned, in which case `MILLISECS_PER_BLOCK` and
	/// `SLOT_DURATION` should have the same value.
	///
	/// <https://research.web3.foundation/en/latest/polkadot/block-production/Babe.html#-6.-practical-results>
	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 2 * MINUTES;
	pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = EPOCH_DURATION_IN_BLOCKS;

	pub const UPDATE_DURATION: BlockNumber = EPOCH_DURATION_IN_SLOTS * 3;

	pub const SESSIONS_PER_ERA: sp_staking::SessionIndex = 3;
	pub const BONDING_DURATION: pallet_staking::EraIndex = 24 * 8;
	pub const SLASH_DEFER_DURATION: pallet_staking::EraIndex = 24 * 2;
	pub const REPORT_LONGEVITY: u64 =
		BONDING_DURATION as u64 * SESSIONS_PER_ERA as u64 * EPOCH_DURATION_IN_SLOTS as u64;

	pub const TERM_DURATION: BlockNumber = 2 * MINUTES;
	pub const COUNCIL_MOTION_DURATION: BlockNumber = 10 * MINUTES;
	pub const TECHNICAL_MOTION_DURATION: BlockNumber = 10 * MINUTES;
	pub const ENACTMENT_PERIOD: BlockNumber = 1 * MINUTES;
	pub const LAUNCH_PERIOD: BlockNumber = 15 * MINUTES;
	pub const VOTING_PERIOD: BlockNumber = 10 * MINUTES;
	pub const FAST_TRACK_VOTING_PERIOD: BlockNumber = 2 * MINUTES;
	pub const COOLOFF_PERIOD: BlockNumber = 60 * MINUTES;

	pub const ALLOWED_PROPOSAL_PERIOD: BlockNumber = 14;
	pub const SPEND_PERIOD: BlockNumber = 2 * MINUTES;
	pub const BOUNTY_DEPOSIT_PAYOUT_DELAY: BlockNumber = 1 * MINUTES;
	pub const TIP_COUNTDOWN: BlockNumber = 1 * MINUTES;
	pub const BOUNTY_UPDATE_PERIOD: BlockNumber = 5 * MINUTES;
}

#[allow(dead_code)]
/// Time.
pub mod time_prod {
	use node_primitives::{BlockNumber, Moment};

	// These time units are defined in number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	pub const DEFAULT_ASSET_LIFETIME: BlockNumber = DAYS * 28;
	/// Since BABE is probabilistic this is the average expected block time that
	/// we are targeting. Blocks will be produced at a minimum duration defined
	/// by `SLOT_DURATION`, but some slots will not be allocated to any
	/// authority and hence no block will be produced. We expect to have this
	/// block time on average following the defined slot duration and the value
	/// of `c` configured for BABE (where `1 - c` represents the probability of
	/// a slot being empty).
	/// This value is only used indirectly to define the unit constants below
	/// that are expressed in blocks. The rest of the code should use
	/// `SLOT_DURATION` instead (like the Timestamp pallet for calculating the
	/// minimum period).
	///
	/// If using BABE with secondary slots (default) then all of the slots will
	/// always be assigned, in which case `MILLISECS_PER_BLOCK` and
	/// `SLOT_DURATION` should have the same value.
	///
	/// <https://research.web3.foundation/en/latest/polkadot/block-production/Babe.html#-6.-practical-results>
	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
	pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = EPOCH_DURATION_IN_BLOCKS;

	pub const UPDATE_DURATION: BlockNumber = EPOCH_DURATION_IN_SLOTS * 6;
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const SESSIONS_PER_ERA: sp_staking::SessionIndex = 6;
	pub const BONDING_DURATION: pallet_staking::EraIndex = 24 * 28;
	pub const SLASH_DEFER_DURATION: pallet_staking::EraIndex = 24 * 27;
	pub const REPORT_LONGEVITY: u64 =
		BONDING_DURATION as u64 * SESSIONS_PER_ERA as u64 * EPOCH_DURATION_IN_SLOTS as u64;

	pub const TERM_DURATION: BlockNumber = 7 * DAYS;
	pub const COUNCIL_MOTION_DURATION: BlockNumber = 5 * DAYS;
	pub const TECHNICAL_MOTION_DURATION: BlockNumber = 5 * DAYS;
	pub const ENACTMENT_PERIOD: BlockNumber = 30 * 24 * 60 * MINUTES;
	pub const LAUNCH_PERIOD: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const VOTING_PERIOD: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const FAST_TRACK_VOTING_PERIOD: BlockNumber = 3 * 24 * 60 * MINUTES;
	pub const COOLOFF_PERIOD: BlockNumber = 28 * 24 * 60 * MINUTES;

	pub const ALLOWED_PROPOSAL_PERIOD: BlockNumber = 24 * DAYS;
	pub const SPEND_PERIOD: BlockNumber = 28 * DAYS;
	pub const BOUNTY_DEPOSIT_PAYOUT_DELAY: BlockNumber = 1 * DAYS;
	pub const TIP_COUNTDOWN: BlockNumber = 1 * DAYS;
	pub const BOUNTY_UPDATE_PERIOD: BlockNumber = 14 * DAYS;
}
