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

pub use pallet::*;

pub mod weights;
pub use weights::WeightInfo;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

use sp_runtime::{
	RuntimeDebug,
};

use frame_support::{
	pallet_prelude::DispatchResult,
	traits::{Currency},
};

type BalanceOf<T> =
	<<T as pallet_poc_staking::Config>::StakingCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[codec(dumb_trait_bound)]
pub struct MiningInfo<AccountId> {
	pub miner: Option<AccountId>,
    pub best_dl: u64,
    pub block: u64,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct MiningHistory<Balance, BlockNumber> {
	total_num: u64,
	history: Vec<(BlockNumber, Balance)>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct Difficulty {
	pub base_target: u64,
    pub net_difficulty: u64,
    // the block height of adjust difficulty
    pub block: u64,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::{pallet_prelude::*};
	use sp_runtime::traits::{SaturatedConversion};

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_poc_staking::Config
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// GENESIS_BASE_TARGET
		type GenesisBaseTarget: Get<u64>;

		type WeightInfo: WeightInfo;
	}

	/// Info on all of the DiskOf.
	#[pallet::storage]
	#[pallet::getter(fn disk_of)]
	pub(crate) type History<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, MiningHistory<BalanceOf<T>, T::BlockNumber>>;

	/// exposed miners(hope someone to stake him).
	#[pallet::storage]
	#[pallet::getter(fn target_info)]
	pub(super) type TargetInfo<T> =
		StorageValue<_, Vec<Difficulty>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Minning { miner: T::AccountId, deadline: u64 },
		SetDifficulty { base_target: u64 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// the difficulty up max value.
		DifficultyIsTooLarge,
		/// the difficulty should not zero.
		DifficultyIsZero,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			Weight::from_ref_time(0 as u64)
		}

		fn on_finalize(_n: BlockNumberFor<T>) {}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// register
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_difficulty())]
		pub fn set_difficulty(origin: OriginFor<T>, difficulty: u64) -> DispatchResult {

        	ensure_root(origin)?;

        	ensure!(difficulty != 0u64, Error::<T>::DifficultyIsZero);

			ensure!(difficulty <= T::GenesisBaseTarget::get(), Error::<T>::DifficultyIsTooLarge);

        	let base_target = T::GenesisBaseTarget::get() / difficulty;

        	let now = pallet_poc_staking::Pallet::<T>::now().saturated_into::<u64>();
        	Self::append_target_info(Difficulty{
                    block: now,
                    base_target: base_target,
                    net_difficulty: T::GenesisBaseTarget::get() / base_target,
                });

			Self::deposit_event(Event::<T>::SetDifficulty { base_target });

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {

	fn append_target_info(difficulty: Difficulty) {

		let mut old_target_info_vec = TargetInfo::<T>::get();
		let len = old_target_info_vec.len();
		old_target_info_vec.push(difficulty);
		if len >= 50 {
			let new_target_info = old_target_info_vec.split_off(len - 50);
			old_target_info_vec = new_target_info;
		}

		TargetInfo::<T>::put(old_target_info_vec);
	}

}
