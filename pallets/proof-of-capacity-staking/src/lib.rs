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

use frame_support::ensure;
pub use pallet::*;

pub mod traits;
pub mod weights;
pub use traits::PocHandler;
pub use weights::WeightInfo;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, SaturatedConversion, Saturating},
	Percent, RuntimeDebug,
};
use sp_std::{collections::btree_set::BTreeSet, result};

use frame_support::{
	pallet_prelude::{DispatchError, DispatchResult},
	traits::{
		Currency, Get, LockIdentifier, LockableCurrency, OnUnbalanced, ReservableCurrency,
		WithdrawReasons,
	},
};

use kumandra_primitives::GIB;

const STAKINGID: LockIdentifier = *b"pocstake";

pub type BalanceOf<T> =
	<<T as Config>::StakingCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type NegativeImbalanceOf<T> = <<T as Config>::StakingCurrency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

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

		type RecommendMaxNumber: Get<usize>;

		type PocHandler: PocHandler<Self::AccountId>;

		type ChillDuration: Get<Self::BlockNumber>;

		type StakingLockExpire: Get<Self::BlockNumber>;

		type StakingSlash: OnUnbalanced<NegativeImbalanceOf<Self>>;

		type StakingDeposit: Get<BalanceOf<Self>>;

		type PocStakingMinAmount: Get<BalanceOf<Self>>;

		type StakerMaxNumber: Get<usize>;

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

	/// is in the chill time(only miners can update their info).
	#[pallet::storage]
	#[pallet::getter(fn is_chill_time)]
	pub(super) type IsChillTime<T> = StorageValue<_, bool, ValueQuery, DefaultChillTime>;

	#[pallet::type_value]
	pub fn DefaultChillTime() -> bool {
		true.into()
	}

	/// is in the chill time(only miners can update their info).
	#[pallet::storage]
	#[pallet::getter(fn chill_time)]
	pub(super) type ChillTime<T: Config> =
		StorageValue<_, (T::BlockNumber, T::BlockNumber), ValueQuery>;

	/// the total number of mining miners.
	#[pallet::storage]
	#[pallet::getter(fn mining_num)]
	pub(super) type MiningNum<T> = StorageValue<_, u64, ValueQuery>;

	/// Locks
	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub(super) type Locks<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<(T::BlockNumber, BalanceOf<T>)>>;

	/// the miners of the user that help stake.
	#[pallet::storage]
	#[pallet::getter(fn miners_of)]
	pub(super) type MinersOf<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<T::AccountId>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ExitStaking { staker: T::AccountId, miner: T::AccountId },
		Register { miner: T::AccountId, disk: u64 },
		RequestDownFromList { miner: T::AccountId },
		RemoveStaker { miner: T::AccountId, staker: T::AccountId },
		RequestUpToList { miner: T::AccountId, amount: BalanceOf<T> },
		RestartMining { miner: T::AccountId },
		Staking { who: T::AccountId, miner: T::AccountId, amount: BalanceOf<T> },
		StopMining { miner: T::AccountId },
		Unlock { staker: T::AccountId },
		UpdateNumericId { miner: T::AccountId, pid: u128 },
		UpdatePlotSize { miner: T::AccountId, disk: GIB },
		UpdateProportion { miner: T::AccountId, proportion: Percent },
		UpdateRewardDest { miner: T::AccountId, dest: T::AccountId },
		UpdateStaking { staker: T::AccountId, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// the miner already register.
		AlreadyRegister,
		/// the user already staking.
		AlreadyStaking,
		/// miner already stop mining.
		AlreadyStopMining,
		/// you should add the amount.
		AmountTooLow,
		/// amount not enough.
		AmountNotEnough,
		/// in chill time.
		ChillTime,
		/// you did not stop mining.
		MiningNotStop,
		/// not in chill time.
		NotChillTime,
		/// the numeric id is in using.
		NumericIdInUsing,
		/// not in the recommend list.
		NotInList,
		/// the miner is not register.
		NotRegister,
		/// not the staker of this miner.
		NotYourStaker,
		/// over flow.
		Overflow,
		/// plot size should not 0.
		PlotSizeIsZero,
		/// you staking amount too low
		StakingAmountooLow,
		/// the satkers number of this miner is up the max value.
		StakerNumberToMax,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// dummy `on_initialize` to return the weight used in `on_finalize`.
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let _ = Self::update_chill();
			// weight of `on_finalize`
			Weight::from_ref_time(0 as u64)
		}

		/// # <weight>
		/// - `O(1)`
		/// - 1 storage deletion (codec `O(1)`).
		/// # </weight>
		fn on_finalize(_n: BlockNumberFor<T>) {
			let num = <MiningMiners<T>>::get().len() as u64;
			MiningNum::<T>::put(num)
		}
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

			DeclaredCapacity::<T>::mutate(|h| *h += disk);

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

		/// request to expose in recommend list.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::request_up_to_list())]
		pub fn request_up_to_list(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			ensure!(Self::is_can_mining(miner.clone())?, Error::<T>::NotRegister);

			Self::sort_account_by_amount(miner.clone(), amount)?;

			Self::deposit_event(Event::<T>::RequestUpToList { miner, amount });

			Ok(())
		}

		/// request to down from the recommended list
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::request_down_from_list())]
		pub fn request_down_from_list(origin: OriginFor<T>) -> DispatchResult {
			let miner = ensure_signed(origin)?;
			let mut list = RecommendList::<T>::get();
			if let Some(pos) = list.iter().position(|h| h.0 == miner) {
				let amount = list.remove(pos).1;

				T::StakingCurrency::unreserve(&miner, amount);

				let now = Self::now();

				let expire = now.saturating_add(T::RecommendLockExpire::get());
				Self::lock_add_amount(miner.clone(), amount, expire);

				RecommendList::<T>::put(list);
			} else {
				return Err(Error::<T>::NotInList)?;
			}

			Self::deposit_event(Event::<T>::RequestDownFromList { miner });

			Ok(())
		}

		/// the miner modify income address.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::update_reward_dest())]
		pub fn update_reward_dest(origin: OriginFor<T>, dest: T::AccountId) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			ensure!(Self::is_register(miner.clone()), Error::<T>::NotRegister);

			DiskOf::<T>::mutate(miner.clone(), |h| {
				if let Some(i) = h {
					i.reward_dest = dest.clone();
				}
			});

			Self::deposit_event(Event::<T>::UpdateRewardDest { miner, dest });

			Ok(())
		}

		/// the miner modify plot id.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::update_numeric_id())]
		pub fn update_numeric_id(origin: OriginFor<T>, numeric_id: u128) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			let pid = numeric_id;

			ensure!(Self::is_register(miner.clone()), Error::<T>::NotRegister);

			ensure!(
				!(AccountIdOfPid::<T>::contains_key(pid)
					&& AccountIdOfPid::<T>::get(pid).unwrap() != miner.clone()),
				Error::<T>::NumericIdInUsing
			);

			let old_pid = DiskOf::<T>::get(miner.clone()).unwrap().numeric_id;

			AccountIdOfPid::<T>::remove(old_pid);

			DiskOf::<T>::mutate(miner.clone(), |h| {
				if let Some(i) = h {
					i.numeric_id = pid;
				}
			});

			<AccountIdOfPid<T>>::insert(pid, miner.clone());

			Self::deposit_event(Event::<T>::UpdateNumericId { miner, pid });

			Ok(())
		}

		/// the miner modify the plot size.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::update_plot_size())]
		pub fn update_plot_size(origin: OriginFor<T>, plot_size: GIB) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			let kib = plot_size;

			let disk = kib.checked_mul((1024 * 1024 * 1024) as GIB).ok_or(Error::<T>::Overflow)?;

			ensure!(disk != 0 as GIB, Error::<T>::PlotSizeIsZero);

			ensure!(Self::is_chill_time(), Error::<T>::ChillTime);

			T::PocHandler::remove_history(miner.clone());

			let now = Self::now();

			ensure!(Self::is_register(miner.clone()), Error::<T>::NotRegister);

			<DiskOf<T>>::mutate(miner.clone(), |h| {
				if let Some(i) = h {
					if i.is_stop == false {
						DeclaredCapacity::<T>::mutate(|h| *h -= i.plot_size);
						i.plot_size = disk;
						DeclaredCapacity::<T>::mutate(|h| *h += i.plot_size);
						i.update_time = now;
					} else {
						i.plot_size = disk;
						i.update_time = now;
					}
				}
			});

			Self::deposit_event(Event::<T>::UpdatePlotSize { miner, disk });

			Ok(())
		}

		/// the miner stop the machine.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::stop_mining())]
		pub fn stop_mining(origin: OriginFor<T>) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			Self::is_can_mining(miner.clone())?;

			<DiskOf<T>>::mutate(miner.clone(), |h| {
				if let Some(x) = h {
					x.is_stop = true;
					DeclaredCapacity::<T>::mutate(|h| *h -= x.plot_size);
					MiningMiners::<T>::mutate(|h| h.remove(&miner));
				}
			});

			Self::deposit_event(Event::<T>::StopMining { miner });

			Ok(())
		}

		/// the miner restart mining.
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::restart_mining())]
		pub fn restart_mining(origin: OriginFor<T>) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			ensure!(Self::is_register(miner.clone()), Error::<T>::NotRegister);

			ensure!(
				DiskOf::<T>::get(miner.clone()).unwrap().is_stop == true,
				Error::<T>::MiningNotStop
			);
			DiskOf::<T>::mutate(miner.clone(), |h| {
				if let Some(x) = h {
					let now = Self::now();
					x.update_time = now;
					x.is_stop = false;
					DeclaredCapacity::<T>::mutate(|h| *h += x.plot_size);
					MiningMiners::<T>::mutate(|h| h.insert(miner.clone()));
				}
			});
			T::PocHandler::remove_history(miner.clone());

			Self::deposit_event(Event::<T>::RestartMining { miner });

			Ok(())
		}

		/// the delete him staker.
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_staker())]
		pub fn remove_staker(origin: OriginFor<T>, staker: T::AccountId) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			Self::update_staking_info(miner.clone(), staker.clone(), Operate::Sub, None, true)?;

			Self::staker_remove_miner(staker.clone(), miner.clone());

			Self::deposit_event(Event::<T>::RemoveStaker { miner, staker });

			Ok(())
		}

		/// the user stake for miners.
		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::staking())]
		pub fn staking(
			origin: OriginFor<T>,
			miner: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::is_can_mining(miner.clone())?;

			ensure!(!IsChillTime::<T>::get(), Error::<T>::ChillTime);

			ensure!(amount >= T::PocStakingMinAmount::get(), Error::<T>::StakingAmountooLow);

			if Self::staker_pos(miner.clone(), who.clone()).is_some() {
				return Err(Error::<T>::AlreadyStaking)?;
			}

			let bond = amount.checked_add(&T::StakingDeposit::get()).ok_or(Error::<T>::Overflow)?;

			let staker_info = (who.clone(), amount.clone(), T::StakingDeposit::get());

			let mut staking_info = StakingInfoOf::<T>::get(&miner).unwrap();

			ensure!(
				staking_info.others.len() < T::StakerMaxNumber::get(),
				Error::<T>::StakerNumberToMax
			);

			let total_amount = staking_info.clone().total_staking;

			let now_amount = total_amount.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

			T::StakingCurrency::reserve(&who, bond)?;

			staking_info.total_staking = now_amount;

			staking_info.others.push(staker_info);

			StakingInfoOf::<T>::insert(miner.clone(), staking_info);

			MinersOf::<T>::mutate(who.clone(), |h| h.push(miner.clone()));

			Self::deposit_event(Event::<T>::Staking { who, miner, amount });

			Ok(())
		}

		/// users update their staking amount.
		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::update_staking())]
		pub fn update_staking(
			origin: OriginFor<T>,
			miner: T::AccountId,
			operate: Operate,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let staker = ensure_signed(origin)?;

			Self::update_staking_info(miner, staker.clone(), operate, Some(amount), false)?;

			Self::deposit_event(Event::<T>::UpdateStaking { staker, amount });

			Ok(())
		}

		/// unlock
		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::update_staking())]
		pub fn unlock(origin: OriginFor<T>) -> DispatchResult {
			let staker = ensure_signed(origin)?;
			Self::lock_sub_amount(staker.clone());
			Self::deposit_event(Event::<T>::Unlock { staker });

			Ok(())
		}

		/// the user exit staking.
		#[pallet::call_index(12)]
		#[pallet::weight(<T as Config>::WeightInfo::exit_staking())]
		pub fn exit_staking(origin: OriginFor<T>, miner: T::AccountId) -> DispatchResult {
			let staker = ensure_signed(origin)?;
			Self::update_staking_info(miner.clone(), staker.clone(), Operate::Sub, None, false)?;
			Self::staker_remove_miner(staker.clone(), miner.clone());
			Self::deposit_event(Event::<T>::ExitStaking { staker, miner });

			Ok(())
		}

		/// miners update their mining reward proportion.
		#[pallet::call_index(13)]
		#[pallet::weight(<T as Config>::WeightInfo::update_proportion())]
		pub fn update_proportion(origin: OriginFor<T>, proportion: Percent) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			ensure!(IsChillTime::<T>::get(), Error::<T>::NotChillTime);

			Self::is_can_mining(miner.clone())?;

			let mut staking_info = <StakingInfoOf<T>>::get(miner.clone()).unwrap();

			staking_info.miner_proportion = proportion.clone();

			<StakingInfoOf<T>>::insert(miner.clone(), staking_info);

			Self::deposit_event(Event::<T>::UpdateProportion { miner, proportion });

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

	fn is_can_mining(miner: T::AccountId) -> result::Result<bool, DispatchError> {
		ensure!(Self::is_register(miner.clone()), Error::<T>::NotRegister);

		ensure!(!DiskOf::<T>::get(&miner).unwrap().is_stop, Error::<T>::AlreadyStopMining);

		Ok(true)
	}

	fn staker_pos(miner: T::AccountId, staker: T::AccountId) -> Option<usize> {
		let staking_info = StakingInfoOf::<T>::get(&miner).unwrap();
		let others = staking_info.others;
		let pos = others.iter().position(|h| h.0 == staker);
		pos
	}

	fn staker_remove_miner(staker: T::AccountId, miner: T::AccountId) {
		MinersOf::<T>::mutate(staker.clone(), |miners| {
			miners.retain(|h| h != &miner);
		});
	}

	fn update_chill() -> DispatchResult {
		let now = Self::now();

		let era_start_time = <pallet_staking::Pallet<T>>::current_era();

		let chill_duration = T::ChillDuration::get(); // 一个number of blocks in session

		let era = chill_duration * era_start_time.unwrap().saturated_into::<T::BlockNumber>(); // 一个number of blocks in era

		// Get the blocks consumed by the era
		let num = now % era;
		let num1 = now / era;

		if num < chill_duration {
			let start = num1 * era;
			let end = num1 * era + chill_duration;
			<ChillTime<T>>::put((start, end));
			<IsChillTime<T>>::put(true);
		} else {
			let start = (num1 + 1u32.saturated_into::<T::BlockNumber>()) * era;
			let end = (num1 + 1u32.saturated_into::<T::BlockNumber>()) * era + chill_duration;
			<ChillTime<T>>::put((start, end));

			<IsChillTime<T>>::put(false);
		}

		Ok(())
	}

	fn update_staking_info(
		miner: T::AccountId,
		staker: T::AccountId,
		operate: Operate,
		amount_opt: Option<BalanceOf<T>>,
		is_slash: bool,
	) -> DispatchResult {
		ensure!(Self::is_register(miner.clone()), Error::<T>::NotRegister);

		let amount: BalanceOf<T>;

		if let Some(pos) = Self::staker_pos(miner.clone(), staker.clone()) {
			let mut staking_info = StakingInfoOf::<T>::get(&miner).unwrap();

			let mut staker_info = staking_info.others.remove(pos);

			if amount_opt.is_none() {
				amount = staker_info.1.clone()
			} else {
				amount = amount_opt.unwrap()
			}

			match operate {
				Operate::Add => {
					let bond = staker_info.1.clone();
					let now_bond = bond.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
					let total_staking = staking_info.total_staking;
					let now_staking =
						total_staking.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
					T::StakingCurrency::reserve(&staker, amount)?;

					staker_info.1 = now_bond;

					staking_info.total_staking = now_staking;
				}

				_ => {
					let bond = staker_info.1.clone();
					let now_bond = bond.checked_sub(&amount).ok_or(Error::<T>::Overflow)?;
					let total_staking = staking_info.total_staking;
					let now_staking =
						total_staking.checked_sub(&amount).ok_or(Error::<T>::Overflow)?;

					T::StakingCurrency::unreserve(&staker, amount);

					let now = Self::now();
					let expire = now.saturating_add(T::StakingLockExpire::get());
					Self::lock_add_amount(staker.clone(), amount, expire);

					staker_info.1 = now_bond;

					staking_info.total_staking = now_staking;
				}
			}

			if staker_info.1 == <BalanceOf<T>>::from(0u32) {
				if is_slash {
					T::StakingSlash::on_unbalanced(
						T::StakingCurrency::slash_reserved(&staker, staker_info.2.clone()).0,
					);
				} else {
					T::StakingCurrency::unreserve(&staker, staker_info.2.clone());
				}
				Self::staker_remove_miner(staker.clone(), miner.clone());
			} else {
				staking_info.others.push(staker_info);
			}

			<StakingInfoOf<T>>::insert(&miner, staking_info);
		} else {
			return Err(Error::<T>::NotYourStaker)?;
		}

		Ok(())
	}

	fn sort_account_by_amount(
		miner: T::AccountId,
		mut amount: BalanceOf<T>,
	) -> result::Result<(), DispatchError> {
		let mut old_list = RecommendList::<T>::get();

		let mut miner_old_info: Option<(T::AccountId, BalanceOf<T>)> = None;

		if let Some(pos) = old_list.iter().position(|h| h.0 == miner.clone()) {
			miner_old_info = Some(old_list.remove(pos));
		}

		if miner_old_info.is_some() {
			let old_amount = miner_old_info.clone().unwrap().1;

			ensure!(T::StakingCurrency::can_reserve(&miner, amount), Error::<T>::AmountNotEnough);

			T::StakingCurrency::unreserve(&miner, old_amount);

			amount = amount + old_amount;
		}

		if old_list.len() == 0 {
			Self::sort_after(miner, amount, 0, old_list)?;
		} else {
			let mut index = 0;
			for i in old_list.iter() {
				if i.1 >= amount {
					index += 1;
				} else {
					break;
				}
			}

			Self::sort_after(miner, amount, index, old_list)?;
		}

		Ok(())
	}

	fn sort_after(
		miner: T::AccountId,
		amount: BalanceOf<T>,
		index: usize,
		mut old_list: Vec<(T::AccountId, BalanceOf<T>)>,
	) -> result::Result<(), DispatchError> {
		if index < T::RecommendMaxNumber::get() {
			T::StakingCurrency::reserve(&miner, amount)?;

			old_list.insert(index, (miner, amount));
		}

		if old_list.len() >= T::RecommendMaxNumber::get() {
			let abandon = old_list.split_off(T::RecommendMaxNumber::get());

			for i in abandon {
				T::StakingCurrency::unreserve(&i.0, i.1);
				let now = Self::now();
				let expire = now.saturating_add(T::RecommendLockExpire::get());

				Self::lock_add_amount(i.0, i.1, expire);
			}
		}

		<RecommendList<T>>::put(old_list);

		if index >= T::RecommendMaxNumber::get() {
			return Err(Error::<T>::AmountTooLow)?;
		}

		Ok(())
	}

	fn lock_add_amount(who: T::AccountId, amount: BalanceOf<T>, expire: T::BlockNumber) {
		Self::lock(who.clone(), Operate::Add, amount);
		let locks_opt = Locks::<T>::get(who.clone());
		if locks_opt.is_some() {
			let mut locks = locks_opt.unwrap();
			locks.push((expire, amount));
			<Locks<T>>::insert(who, locks);
		} else {
			let locks = vec![(expire, amount)];
			Locks::<T>::insert(who, locks);
		}
	}

	fn lock_sub_amount(who: T::AccountId) {
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

	/// todo: add and sub balance more property
	fn lock(who: T::AccountId, operate: Operate, amount: BalanceOf<T>) {
		let locks_opt = Locks::<T>::get(who.clone());
		let reasons = WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE;
		match operate {
			Operate::Sub => {
				if locks_opt.is_none() {
				} else {
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
