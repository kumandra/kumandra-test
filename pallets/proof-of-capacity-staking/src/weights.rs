// Copyright 2022-2023 Smallworld Kumandra.
// This file is part of Kumandra.

// Kumandra is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Kumandra is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Kumandra. If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
	fn register() -> Weight;
	fn request_down_from_list() -> Weight;
	fn request_up_to_list() -> Weight;
	fn update_reward_dest() -> Weight;
	fn update_numeric_id() -> Weight;
	fn update_plot_size() -> Weight;
	fn stop_mining() -> Weight;
	fn staking() -> Weight;
	fn restart_mining() -> Weight;
	fn remove_staker() -> Weight;
	fn update_staking() -> Weight;
	fn unlock() -> Weight;
	fn exit_staking() -> Weight;
	fn update_proportion() -> Weight;
}

/// Weights for pallet_lottery using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn register() -> Weight {
		Weight::from_ref_time(53_225_000)
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	fn request_down_from_list() -> Weight {
		Weight::from_ref_time(53_225_000)
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	fn request_up_to_list() -> Weight {
		Weight::from_ref_time(53_225_000)
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	fn update_reward_dest() -> Weight {
		Weight::from_ref_time(53_225_000)
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	fn update_numeric_id() -> Weight {
		Weight::from_ref_time(53_225_000)
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	fn update_plot_size() -> Weight {
		Weight::from_ref_time(53_225_000)
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	fn stop_mining() -> Weight {
		Weight::from_ref_time(53_225_000)
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	fn restart_mining() -> Weight {
		Weight::from_ref_time(53_225_000)
		.saturating_add(T::DbWeight::get().reads(6))
		.saturating_add(T::DbWeight::get().writes(4))
	}

	fn remove_staker() -> Weight {
		Weight::from_ref_time(53_225_000)
		.saturating_add(T::DbWeight::get().reads(6))
		.saturating_add(T::DbWeight::get().writes(4))
	}

	fn update_staking() -> Weight {
		Weight::from_ref_time(53_225_000)
		.saturating_add(T::DbWeight::get().reads(6))
		.saturating_add(T::DbWeight::get().writes(4))
	}

	fn staking() -> Weight {
		Weight::from_ref_time(53_225_000)
		.saturating_add(T::DbWeight::get().reads(6))
		.saturating_add(T::DbWeight::get().writes(4))
	}

	fn unlock() -> Weight {
		Weight::from_ref_time(53_225_000)
		.saturating_add(T::DbWeight::get().reads(6))
		.saturating_add(T::DbWeight::get().writes(4))
	}

	fn exit_staking() -> Weight {
		Weight::from_ref_time(53_225_000)
		.saturating_add(T::DbWeight::get().reads(6))
		.saturating_add(T::DbWeight::get().writes(4))
	}

	fn update_proportion() -> Weight {
		Weight::from_ref_time(53_225_000)
		.saturating_add(T::DbWeight::get().reads(6))
		.saturating_add(T::DbWeight::get().writes(4))
	}
}
