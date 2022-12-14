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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};

use sp_runtime::Percent;
use sp_std::prelude::*;

use frame_support::{BoundedBTreeMap, pallet_prelude::*};
use frame_system::pallet_prelude::*;

use pallet_support::primitives::GIB;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::traits::{
		Currency, Get, LockableCurrency, OnUnbalanced, ReservableCurrency,
	};
	use pallet_support::traits::PocHandler;

	pub type BalanceOf<T> = <<T as Config>::StakingCurrency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	pub type NegativeImbalanceOf<T> = <<T as Config>::StakingCurrency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_timestamp::Config
		+ pallet_balances::Config
		+ pallet_babe::Config
		+ pallet_staking::Config
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type ChillDuration: Get<Self::BlockNumber>;

		type StakingCurrency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;

		type StakingDeposit: Get<BalanceOf<Self>>;

		type StakingSlash: OnUnbalanced<NegativeImbalanceOf<Self>>;

		type StakerMaxNumber: Get<u32>;

		type StakingLockExpire: Get<Self::BlockNumber>;

		type PocHandler: PocHandler<Self::AccountId>;

		type RecommendLockExpire: Get<Self::BlockNumber>;

		type RecommendMaxNumber: Get<u32>;
	}

	/// Miner's machine information
	#[derive(
		Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo,
	)]
	pub struct MachineInfo<BlockNumber, AccountId> {
		/// disk space
		pub plot_size: GIB,
		/// disk id
		pub numeric_id: u128,
		/// update time
		pub update_time: BlockNumber,
		/// Whether the machine is running
		pub is_stop: bool,
		/// reward address
		pub reward_dest: AccountId,
	}

	/// Staking
	#[derive(
		Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo,
	)]
	pub struct StakingInfo<AccountId, Balance> {
		/// Miner
		pub miner: AccountId,
		/// Profit sharing ratio of miners
		pub miner_proportion: Percent,
		/// Total staking amount
		pub total_staking: Balance,
		// /// Other staking
		pub others: BoundedVec<(AccountId, Balance, Balance), ConstU32<25>>,
	}

	/// Operate
	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, MaxEncodedLen)]
	pub enum Oprate {
		Add,
		Sub,
	}

	impl Default for Oprate {
		fn default() -> Self {
			Self::Add
		}
	}

	/// the machine info of miners.
	#[pallet::storage]
	#[pallet::getter(fn disk_of)]
	pub(super) type DiskOf<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		Option<MachineInfo<T::BlockNumber, T::AccountId>>,
		ValueQuery,
	>;

	/// is in the chill time(only miners can update their info).
	#[pallet::storage]
	#[pallet::getter(fn is_chill_time)]
	pub(super) type IsChillTime<T> = StorageValue<_, bool, ValueQuery, DefaultChillTime>;

	#[pallet::type_value]
	pub fn DefaultChillTime() -> bool {
		true.into()
	}

	/// the staking info of miners.
	#[pallet::storage]
	#[pallet::getter(fn staking_info_of)]
	pub(super) type StakingInfoOf<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		Option<StakingInfo<T::AccountId, BalanceOf<T>>>,
		ValueQuery,
	>;

	/// the miners of the user that help stake.
	#[pallet::storage]
	#[pallet::getter(fn miners_of)]
	pub(super) type MinersOf<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::AccountId, ConstU32<25>>,
		ValueQuery,
	>;

	/// whose plot id?.
	#[pallet::storage]
	#[pallet::getter(fn accouont_id_of_pid)]
	pub(super) type AccountIdOfPid<T: Config> =
		StorageMap<_, Twox64Concat, u128, Option<T::AccountId>, ValueQuery>;

	/// exposed miners(hope someone to stake him).
	#[pallet::storage]
	#[pallet::getter(fn recommend_list)]
	pub(super) type RecommendList<T: Config> = StorageValue<
		_,
		BoundedVec<(T::AccountId, BalanceOf<T>), T::RecommendMaxNumber>,
		ValueQuery,
	>;

	/// the total declared capacity in the entire network.
	#[pallet::storage]
	#[pallet::getter(fn declared_capacity)]
	pub(super) type DeclaredCapacity<T> = StorageValue<_, u64, ValueQuery>;

	/// minsers that already registered.
	#[pallet::storage]
	#[pallet::getter(fn miners)]
	pub(super) type Miners<T: Config> = StorageValue<_, BoundedBTreeMap<T::AccountId, (), ConstU32<25>>, ValueQuery>;

	/// Locks
	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub(super) type Locks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		Option<BoundedVec<(T::BlockNumber, BalanceOf<T>), ConstU32<25>>>,
		ValueQuery,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored { something: u32, who: T::AccountId },

		UpdatePlotSize {miner: T::AccountId, disk: GIB },
		Register { miner: T::AccountId, disk: u64 },
		StopMining { miner: T::AccountId },
		RemoveStaker { miner: T::AccountId, staker: T::AccountId },
		Staking { who: T::AccountId, miner: T::AccountId, amount: T::Balance },
		UpdateProportion { miner: T::AccountId, proportion: Percent },
		UpdateStaking { staker: T::AccountId, amount: T::Balance },
		ExitStaking { staker: T::AccountId, miner: T::AccountId },
		UpdateNumericId { miner: T::AccountId, pid: u128 },
		RequestUpToList { miner: T::AccountId, amount: T::Balance },
		RequestDownFromList { miner: T::AccountId },
		Unlock { staker: T::AccountId },
		RestartMining { miner: T::AccountId },
		UpdateRewardDest { miner: T::AccountId , dest: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// the numeric id is in using.
		NumericIdInUsing,
		/// the miner already register.
		AlreadyRegister,
		/// the miner is not register.
		NotRegister,
		/// plot size should not 0.
		PlotSizeIsZero,
		/// in chill time.
		ChillTime,
		/// not in chill time.
		NotChillTime,
		/// miner already stop mining.
		AlreadyStopMining,
		/// not the staker of this miner.
		NotYourStaker,
		/// the user already staking.
		AlreadyStaking,
		/// over flow.
		Overflow,
		/// the satkers number of this miner is up the max value.
		StakerNumberToMax,
		/// amount not enough.
		AmountNotEnough,
		/// not in the recommend list.
		NotInList,
		/// you did not stop mining.
		MiningNotStop,
		/// you should add the amount.
		AmountTooLow,
		/// you staking amount too low
		StakingAmountooLow,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn do_something(origin: OriginFor<T>, _something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let _who = ensure_signed(origin)?;

			Ok(())
		}
	}
}
