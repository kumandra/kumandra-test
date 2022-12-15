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
pub mod traits;
pub use weights::WeightInfo;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

use sp_runtime::{Percent, RuntimeDebug};
use sp_std::collections::btree_set::BTreeSet;

use frame_support::pallet_prelude::DispatchResult;
use frame_support::traits::{
	Currency, LockIdentifier, LockableCurrency, ReservableCurrency, WithdrawReasons,
};

use kumandra_primitives::GIB;

const STAKINGID: LockIdentifier = *b"pocstake";

pub type BalanceOf<T> =
	<<T as Config>::StakingCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[codec(dumb_trait_bound)]
pub struct MachineInfo<AccountId, BlockNumber> {
	/// disk space
	pub plot_size: GIB,
	/// update time
	pub update_time: BlockNumber,
	/// disk id
	pub numeric_id: u128,
	/// Whether the machine is running
	pub is_stop: bool,
	/// reward address
	pub reward_dest: AccountId,
}

/// Staking
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct StakingInfo<AccountId, Balance> {
	/// Miner
	pub miner: AccountId,
	/// Profit sharing ratio of miners
	pub miner_proportion: Percent,
	/// Total staking amount
	pub total_staking: Balance,
	// /// Other staking
	pub others: Vec<(AccountId, Balance, Balance)>,
}

/// Operate
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum Operate {
	Add,
	Sub,
}

impl Default for Operate {
	fn default() -> Self {
		Self::Add
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::{ensure_signed, pallet_prelude::*};

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_staking::Config + pallet_balances::Config
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type StakingCurrency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;

		type RecommendLockExpire: Get<Self::BlockNumber>;

		type WeightInfo: WeightInfo;
	}

	/// Info on all of the DiskOf.
	#[pallet::storage]
	#[pallet::getter(fn disk_of)]
	pub(crate) type DiskOf<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, MachineInfo<T::AccountId, T::BlockNumber>>;

	/// the staking info of miners.
	#[pallet::storage]
	#[pallet::getter(fn staking_info_of)]
	pub(super) type StakingInfoOf<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, StakingInfo<T::AccountId, BalanceOf<T>>>;

	/// whose plot id?.
	#[pallet::storage]
	#[pallet::getter(fn accouont_id_of_pid)]
	pub(super) type AccountIdOfPid<T: Config> = StorageMap<_, Twox64Concat, u128, T::AccountId>;

	/// the total declared capacity in the entire network.
	#[pallet::storage]
	#[pallet::getter(fn declared_capacity)]
	pub(super) type DeclaredCapacity<T> = StorageValue<_, u64, ValueQuery>;

	/// minsers that already registered.
	#[pallet::storage]
	#[pallet::getter(fn miners)]
	pub(super) type Miners<T: Config> = StorageValue<_, BTreeSet<T::AccountId>, ValueQuery>;

	/// miners whom is mining.
	#[pallet::storage]
	#[pallet::getter(fn mining_miners)]
	pub(super) type MiningMiners<T: Config> = StorageValue<_, BTreeSet<T::AccountId>, ValueQuery>;

	/// exposed miners(hope someone to stake him).
	#[pallet::storage]
	#[pallet::getter(fn recommend_list)]
	pub(super) type RecommendList<T: Config> =
		StorageValue<_, Vec<(T::AccountId, BalanceOf<T>)>, ValueQuery>;

	/// Locks
	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub(super) type Locks<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<(T::BlockNumber, BalanceOf<T>)>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Register { miner: T::AccountId, disk: u64 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// the miner already register.
		AlreadyRegister,
		/// miner already stop mining.
		AlreadyStopMining,
		/// the numeric id is in using.
		NumericIdInUsing,
		/// plot size should not 0.
		PlotSizeIsZero,
		/// over flow.
		Overflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// register
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register())]
		pub fn register(
			origin: OriginFor<T>,
			plot_size: GIB,
			numeric_id: u128,
			miner_proportion: u32,
			reward_dest: Option<T::AccountId>,
		) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			let miner_proportion = Percent::from_percent(miner_proportion as u8);

			let kib = plot_size;

			let pid = numeric_id;

			ensure!(kib != 0 as GIB, Error::<T>::PlotSizeIsZero);

			let disk = kib.checked_mul((1024 * 1024 * 1024) as GIB).ok_or(Error::<T>::Overflow)?;

			ensure!(!Self::is_register(miner.clone()), Error::<T>::AlreadyRegister);

			ensure!(!<AccountIdOfPid<T>>::contains_key(pid), Error::<T>::NumericIdInUsing);

			<DeclaredCapacity<T>>::mutate(|h| *h += disk);

			let dest: T::AccountId;
			if reward_dest.is_some() {
				dest = reward_dest.unwrap();
			} else {
				dest = miner.clone();
			}

			let now = Self::now();

			DiskOf::<T>::insert(
				miner.clone(),
				MachineInfo {
					plot_size: disk,
					update_time: now,
					numeric_id: pid,
					is_stop: false,
					reward_dest: dest,
				},
			);

			StakingInfoOf::<T>::insert(
				&miner,
				StakingInfo {
					miner: miner.clone(),
					miner_proportion,
					total_staking: <BalanceOf<T>>::from(0u32),
					others: vec![],
				},
			);

			AccountIdOfPid::<T>::insert(pid, miner.clone());

			Miners::<T>::mutate(|h| h.insert(miner.clone()));

			MiningMiners::<T>::mutate(|h| h.insert(miner.clone()));

			Self::deposit_event(Event::<T>::Register { miner, disk });

			Ok(())
		}

		// register
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::request_down_from_list())]
		pub fn request_down_from_list(origin: OriginFor<T>) -> DispatchResult {
			let miner = ensure_signed(origin)?;
			let mut list = RecommendList::<T>::get();
			if let Some(pos) = list.iter().position(|h| h.0 == miner) {
				let amount = list.remove(pos).1;

				T::StakingCurrency::unreserve(&miner, amount);

				let now = Self::now();

				let expire = now + T::RecommendLockExpire::get();
				Self::lock_add_amount(miner.clone(), amount, expire);

			// <RecommendList<T>>::put(list);
			} else {
				// return Err(Error::<T>::NotInList)?;
			}

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn now() -> T::BlockNumber {
		frame_system::Pallet::<T>::block_number()
	}

	fn is_register(miner: T::AccountId) -> bool {
		if <DiskOf<T>>::contains_key(&miner) && <StakingInfoOf<T>>::contains_key(&miner) {
			true
		} else {
			false
		}
	}

	fn lock_add_amount(who: T::AccountId, amount: BalanceOf<T>, expire: T::BlockNumber) {
		Self::lock(who.clone(), Operate::Add, amount);
		let locks_opt = <Locks<T>>::get(who.clone());
		if locks_opt.is_some() {
			let mut locks = locks_opt.unwrap();
			locks.push((expire, amount));
			<Locks<T>>::insert(who, locks);
		} else {
			let locks = vec![(expire, amount)];
			Locks::<T>::insert(who, locks);
		}
	}

	fn _lock_sub_amount(who: T::AccountId) {
		let now = Self::now();
		<Locks<T>>::mutate(who.clone(), |h_opt| {
			if let Some(h) = h_opt {
				h.retain(|i| {
					if i.0 <= now {
						Self::lock(who.clone(), Operate::Sub, i.1);
						false
					} else {
						true
					}
				});
			}
		});
	}

	fn lock(who: T::AccountId, operate: Operate, amount: BalanceOf<T>) {
		let locks_opt = Locks::<T>::get(who.clone());
		let reasons = WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE;
		match operate {
			Operate::Sub => {
				if locks_opt.is_none() {
				}
				//
				else {
					T::StakingCurrency::set_lock(STAKINGID, &who, amount, reasons);
				}
			}

			Operate::Add => {
				if locks_opt.is_none() {
					T::StakingCurrency::set_lock(STAKINGID, &who, amount, reasons);
				}
				//
				else {
					T::StakingCurrency::extend_lock(STAKINGID, &who, amount, reasons);
				}
			}
		};
	}
}
