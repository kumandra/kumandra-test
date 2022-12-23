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
#![cfg_attr(not(feature = "std"), no_std)]


pub use pallet::*;

pub mod weights;
pub use weights::WeightInfo;

use num_traits::{CheckedDiv, Zero};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

use frame_support::inherent::Vec;
use scale_info::prelude::vec;


use frame_support::traits::Get;
use sp_std::result;

use sp_runtime::{
	traits::{SaturatedConversion, Saturating},
	Percent, RuntimeDebug,
};

use frame_support::{
	dispatch::{DispatchError, DispatchResult},
	traits::{Currency, OnUnbalanced},
};

use conjugate_poc::{
	nonce::noncegen_rust,
	poc_hashing::{calculate_scoop, find_best_deadline_rust},
};

pub const GIB: u64 = 1024 * 1024 * 1024;
pub const MININGEXPIRE: u64 = 2;
pub const SPEED: u64 = 11;

type BalanceOf<T> = <<T as pallet_poc_staking::Config>::StakingCurrency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

type PositiveImbalanceOf<T> = <<T as pallet_poc_staking::Config>::StakingCurrency as Currency<
	<T as frame_system::Config>::AccountId,
>>::PositiveImbalance;

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
	use frame_system::pallet_prelude::*;
	use sp_std::collections::btree_set::BTreeSet;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_poc_staking::Config + pallet_treasury::Config
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type PocAddOrigin: OnUnbalanced<PositiveImbalanceOf<Self>>;

		type GenesisBaseTarget: Get<u64>;

		type TotalMiningReward: Get<BalanceOf<Self>>;

		type MaxDeadlineValue: Get<u64>;

		type ProbabilityDeviationValue: Get<Percent>;

		type BlockTime: Get<u64>;

		type CapacityPrice: Get<BalanceOf<Self>>;

		type WeightInfo: WeightInfo;
	}

	/// Info on all of the DiskOf.
	#[pallet::storage]
	#[pallet::getter(fn history)]
	pub(crate) type History<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, MiningHistory<BalanceOf<T>, T::BlockNumber>>;

	/// the reward history of users.
	#[pallet::storage]
	#[pallet::getter(fn disk_of)]
	pub(crate) type UserRewardHistory<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<(T::BlockNumber, BalanceOf<T>)>>;

	/// exposed miners(hope someone to stake him).
	#[pallet::storage]
	#[pallet::getter(fn active_miners)]
	pub(super) type ActiveMiners<T: Config> =
		StorageValue<_, (u32, BTreeSet<T::AccountId>, u32, BTreeSet<T::AccountId>), ValueQuery>;

	/// difficulties of some duration(50 blocks).
	#[pallet::storage]
	#[pallet::getter(fn target_info)]
	pub(super) type TargetInfo<T> = StorageValue<_, Vec<Difficulty>, ValueQuery>;

	/// deadlines of the mining.
	#[pallet::storage]
	#[pallet::getter(fn dl_info)]
	pub(super) type DlInfo<T: Config> = StorageValue<_, Vec<MiningInfo<T::AccountId>>, ValueQuery>;

	/// the net power(how much capacity)
	#[pallet::storage]
	#[pallet::getter(fn net_power)]
	pub(super) type NetPower<T> = StorageValue<_, u64, ValueQuery>;

	/// how often to adjust difficulty.
	#[pallet::storage]
	#[pallet::getter(fn adjust_difficulty_duration)]
	pub(super) type AdjustDifficultyDuration<T> =
		StorageValue<_, u64, ValueQuery, DefaultDifficulty>;

	#[pallet::type_value]
	pub fn DefaultDifficulty() -> u64 {
		50
	}

	/// how much capacity that one difficulty.
	#[pallet::storage]
	#[pallet::getter(fn capacity_of_per_difficult)]
	pub(super) type CapacityOfPerDifficulty<T> =
		StorageValue<_, u64, ValueQuery, DefaultCapacityOfPerDifficulty>;

	#[pallet::type_value]
	pub fn DefaultCapacityOfPerDifficulty() -> u64 {
		5
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Minning { miner: T::AccountId, deadline: u64 },
		SetAdjustDifficultyDuration { block_num: u64 },
		SetCapacityOfPerDifficulty { capacity: u64 },
		SetDifficulty { base_target: u64 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// the capacity should not empty.
		CapacityIsZero,
		/// data type conversion error
		ConvertErr,
		/// submit deadline up max value.
		DeadlineTooLarge,
		/// the difficulty up max value.
		DifficultyIsTooLarge,
		/// the difficulty should not zero.
		DifficultyIsZero,
		/// 0 can't be a divisor.
		DivZero,
		/// the block number should not zero.
		DurationIsZero,
		/// submit deadline too delay.
		HeightNotInDuration,
		/// not best deadline
		NotBestDeadline,
		/// not register.
		NotRegister,
		/// not your plot id.
		PidErr,
		/// deadline verify failed.
		VerifyFaile,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			if n == T::BlockNumber::from(1u32) {
				Self::append_target_info(Difficulty {
					base_target: T::GenesisBaseTarget::get(),
					net_difficulty: 1,
					block: 1,
				});
			}
			Weight::from_ref_time(0 as u64)
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			let one_hour = (60u64.saturating_div(T::BlockTime::get())).saturating_mul(60u64);
			let one_day = 24 * one_hour;
			let one_year = 365 * one_day;

			if (n % (8 * one_hour).saturated_into::<T::BlockNumber>()).is_zero() {
				ActiveMiners::<T>::mutate(|h| {
					h.2 = h.0.clone();
					h.3 = h.1.clone();
					h.1.clear();
					h.0 = 0u32;
				});
			}
			let current_block = n.saturated_into::<u64>();
			let last_mining_block = Self::get_last_mining_block();

			log::debug!(
				"current-block = {}, last-mining-block = {}",
				current_block,
				last_mining_block
			);

			let reward_result =
				Self::get_reward_amount(one_year.saturated_into::<T::BlockNumber>());

			let reward: BalanceOf<T>;

			if reward_result.is_ok() {
				reward = reward_result.unwrap();
			} else {
				return;
			}

			if (current_block + 1) % MININGEXPIRE == 0 {
				if current_block / MININGEXPIRE == last_mining_block / MININGEXPIRE {
					if let Some(miner_info) = Self::dl_info().last() {
						let miner: Option<T::AccountId> = miner_info.clone().miner;
						if miner.is_some() {
							Self::reward(miner.unwrap(), reward).unwrap();
							log::debug!(
								"<<REWARD>> miner on block {}, last_mining_block {}",
								current_block,
								last_mining_block
							);
						}
					}
				} else {
					Self::treasury_minning(current_block);
					Self::reward_treasury(reward);
				}
			}

			if current_block % AdjustDifficultyDuration::<T>::get() == 0 {
				Self::adjust_difficulty(current_block);
			}

			Self::get_total_capacity();
		}
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
			Self::append_target_info(Difficulty {
				block: now,
				base_target: base_target,
				net_difficulty: T::GenesisBaseTarget::get() / base_target,
			});

			Self::deposit_event(Event::<T>::SetDifficulty { base_target });

			Ok(())
		}

		/// how often to adjust the difficulty.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::set_adjust_difficulty_duration())]
		pub fn set_adjust_difficulty_duration(
			origin: OriginFor<T>,
			block_num: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(block_num > 0u64, Error::<T>::DurationIsZero);
			AdjustDifficultyDuration::<T>::put(block_num);
			Self::deposit_event(Event::<T>::SetAdjustDifficultyDuration { block_num });

			Ok(())
		}

		/// how much capacity that one difficulty.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_capacity_of_per_difficulty())]
		pub fn set_capacity_of_per_difficulty(
			origin: OriginFor<T>,
			capacity: u64,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(capacity != 0u64, Error::<T>::CapacityIsZero);
			CapacityOfPerDifficulty::<T>::put(capacity);

			Self::deposit_event(Event::<T>::SetCapacityOfPerDifficulty { capacity });

			Ok(())
		}

		/// submit deadline.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::mining())]
		pub fn mining(
			origin: OriginFor<T>,
			account_id: u64,
			height: u64,
			sig: [u8; 32],
			nonce: u64,
			deadline: u64,
		) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			<ActiveMiners<T>>::mutate(|h| {
				if h.1.insert(miner.clone()) {
					h.0 += 1;
				}
			});

			log::debug!(
				"miner: {:?},  submit deadline!, height = {}, deadline = {}",
				miner.clone(),
				height,
				deadline
			);

			ensure!(deadline <= T::MaxDeadlineValue::get(), Error::<T>::DeadlineTooLarge);

			ensure!(
				<pallet_poc_staking::Pallet<T>>::is_can_mining(miner.clone())?,
				Error::<T>::NotRegister
			);

			let real_pid = <pallet_poc_staking::Pallet<T>>::disk_of(&miner).unwrap().numeric_id;

			ensure!(real_pid == account_id.into(), Error::<T>::PidErr);

			let current_block = <frame_system::Pallet<T>>::block_number().saturated_into::<u64>();

			log::debug!("starting Verify Deadline !!!");

			if !(current_block / MININGEXPIRE == height / MININGEXPIRE && current_block >= height) {
				log::debug!(
					"expire! ：{:?}, off chain get info block: {:?}, deadline is: {:?}",
					height,
					current_block,
					deadline
				);

				return Err(Error::<T>::HeightNotInDuration)?;
			}

			let dl = Self::dl_info();
			let (block, best_dl) = if let Some(dl_info) = dl.iter().last() {
				(dl_info.clone().block, dl_info.best_dl)
			} else {
				(0, core::u64::MAX)
			};

			// Someone(miner) has mined a better deadline at this mining cycle before.
			if best_dl <= deadline && current_block / MININGEXPIRE == block / MININGEXPIRE {
				log::debug!(
					"not best deadline! best_dl = {}, submit deadline = {}!",
					best_dl,
					deadline
				);

				return Err(Error::<T>::NotBestDeadline)?;
			}

			let verify_ok = Self::verify_dl(account_id, height, sig, nonce, deadline);

			if verify_ok.0 {
				log::debug!("verify is ok!, deadline = {}", deadline);

				if current_block / MININGEXPIRE == block / MININGEXPIRE {
					DlInfo::<T>::mutate(|dl| dl.pop());
				}

				Self::append_dl_info(MiningInfo {
					miner: Some(miner.clone()),
					best_dl: deadline,
					block: current_block,
				});

				Self::deposit_event(Event::<T>::Minning { miner, deadline });
			} else {
				log::debug!(
					"verify failed! deadline = {:?}, target = {:?}, base_target = {:?}",
					verify_ok.1 / verify_ok.2,
					verify_ok.1,
					verify_ok.2
				);
				return Err(Error::<T>::VerifyFaile)?;
			}

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn treasury_minning(current_block: u64) {
		Self::append_dl_info(MiningInfo {
			miner: None,
			best_dl: T::MaxDeadlineValue::get(),
			block: current_block,
		});
		log::debug!("<<REWARD>> treasury on block {}", current_block);
	}

	fn append_dl_info(dl_info: MiningInfo<T::AccountId>) {
		let mut old_dl_info_vec = <DlInfo<T>>::get();
		let len = old_dl_info_vec.len();

		old_dl_info_vec.push(dl_info);

		if len >= 2000 {
			let new_dl_info = old_dl_info_vec.split_off(len - 2000);
			old_dl_info_vec = new_dl_info;
		}

		<DlInfo<T>>::put(old_dl_info_vec);
	}

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

	fn get_last_mining_block() -> u64 {
		let dl = Self::dl_info();
		if let Some(dl) = dl.iter().last() {
			dl.block
		} else {
			0
		}
	}

	fn get_reward_amount(one_year: T::BlockNumber) -> result::Result<BalanceOf<T>, DispatchError> {
		let now = <pallet_poc_staking::Pallet<T>>::now();

		let sub_half_reward_time = 2u32;

		let total_mining_reward = T::TotalMiningReward::get();

		let year = now.checked_div(&T::BlockNumber::from(one_year)).ok_or(Error::<T>::DivZero)?;
		let duration = year / T::BlockNumber::from(sub_half_reward_time);

		let duration =
			<<T as frame_system::Config>::BlockNumber as TryInto<u32>>::try_into(duration)
				.map_err(|_| Error::<T>::ConvertErr)?
				+ 1u32;

		let n_opt = sub_half_reward_time.checked_pow(duration);

		let reward: BalanceOf<T>;

		if n_opt.is_some() {
			let n = <BalanceOf<T>>::from(n_opt.unwrap());

			reward = total_mining_reward
				/ n / 2u32.saturated_into::<BalanceOf<T>>()
				/ Self::block_convert_to_balance(T::BlockNumber::from(one_year))?;

			Ok(reward * MININGEXPIRE.saturated_into::<BalanceOf<T>>())
		} else {
			Ok(<BalanceOf<T>>::from(0u32))
		}
	}

	fn reward(miner: T::AccountId, mut reward: BalanceOf<T>) -> DispatchResult {
		let all_reward = reward.clone();

		let machine_info =
			<pallet_poc_staking::Pallet<T>>::disk_of(&miner).ok_or(Error::<T>::NotRegister)?;
		let disk = machine_info.clone().plot_size;
		let update_time = machine_info.clone().update_time;

		let miner_mining_num = match <History<T>>::get(&miner) {
			Some(h) => h.total_num + 1u64,
			None => 1u64,
		};

		let now = <pallet_poc_staking::Pallet<T>>::now();

		let staking_info_opt = <pallet_poc_staking::Pallet<T>>::staking_info_of(&miner);

		if staking_info_opt.is_some() {
			let total_staking = staking_info_opt.unwrap().total_staking;

			let miner_should_staking_amount =
				disk.saturated_into::<BalanceOf<T>>().saturating_mul(T::CapacityPrice::get())
					/ GIB.saturated_into::<BalanceOf<T>>();

			if miner_should_staking_amount <= total_staking {
				log::debug!("miner's staking enough！staking enough = {:?} ", total_staking);

				let mut net_mining_num = (now - update_time).saturated_into::<u64>() / MININGEXPIRE;

				if net_mining_num < miner_mining_num {
					net_mining_num = miner_mining_num
				}

				log::debug!(
					"miner: {:?}, mining probability: {:?} / {:?}",
					miner.clone(),
					miner_mining_num,
					net_mining_num
				);

				let net_should_staking_total_amount = Self::get_total_capacity()
					.saturated_into::<BalanceOf<T>>()
					.saturating_mul(T::CapacityPrice::get())
					/ GIB.saturated_into::<BalanceOf<T>>();

				if (miner_mining_num
					.saturated_into::<BalanceOf<T>>()
					.saturating_mul(net_should_staking_total_amount)
					> (net_mining_num.saturated_into::<BalanceOf<T>>()
						* miner_should_staking_amount)
						.saturating_add(
							T::ProbabilityDeviationValue::get()
								* (net_mining_num.saturated_into::<BalanceOf<T>>()
									* miner_should_staking_amount),
						)) || ((net_mining_num.saturated_into::<BalanceOf<T>>()
					* miner_should_staking_amount)
					.saturating_sub(
						T::ProbabilityDeviationValue::get()
							* net_mining_num.saturated_into::<BalanceOf<T>>()
							* miner_should_staking_amount,
					) > miner_mining_num
					.saturated_into::<BalanceOf<T>>()
					.saturating_mul(net_should_staking_total_amount))
				{
					log::debug!("Miners: {:?} have a high probability of mining, and you should increase the disk space", miner.clone());

					reward = Percent::from_percent(10) * reward;

					Self::reward_staker(miner.clone(), reward).unwrap();

					Self::reward_treasury(Percent::from_percent(90) * all_reward);
				} else {
					log::debug!("Get all reward.");
					reward = reward;
					Self::reward_staker(miner.clone(), reward).unwrap();
				}
			} else {
				log::debug!("Get 10% reward.");
				reward = Percent::from_percent(10) * reward;
				Self::reward_staker(miner.clone(), reward).unwrap();
				Self::reward_treasury(Percent::from_percent(90) * all_reward);
			}
		} else {
			log::debug!("miner have no staking info.");
			reward = Percent::from_percent(10) * reward;
			Self::reward_staker(miner.clone(), reward).unwrap();
			Self::reward_treasury(Percent::from_percent(90) * all_reward);
		}

		let history_opt = <History<T>>::get(&miner);

		if history_opt.is_some() {
			let mut his = history_opt.unwrap();
			his.total_num = miner_mining_num;
			his.history.push((now, reward));

			if his.history.len() >= 300 {
				let mut old_history = his.history.clone();
				let new_history = old_history.split_off(1);
				his.history = new_history;
			}
			History::<T>::insert(miner.clone(), his);
		} else {
			let history = vec![(now, reward)];
			History::<T>::insert(
				miner.clone(),
				MiningHistory { total_num: miner_mining_num, history: history },
			);
		}

		Ok(())
	}

	fn block_convert_to_balance(n: T::BlockNumber) -> result::Result<BalanceOf<T>, DispatchError> {
		let n_u = <<T as frame_system::Config>::BlockNumber as TryInto<u32>>::try_into(n)
			.map_err(|_| Error::<T>::ConvertErr)?;
		let n_b =
			<BalanceOf<T> as TryFrom<u32>>::try_from(n_u).map_err(|_| Error::<T>::ConvertErr)?;
		Ok(n_b)
	}

	fn reward_treasury(reward: BalanceOf<T>) {
		let account_id = Self::get_treasury_id();
		T::PocAddOrigin::on_unbalanced(T::StakingCurrency::deposit_creating(&account_id, reward));
		// Self::deposit_event(RawEvent::RewardTreasury(account_id, reward));
	}

	fn adjust_difficulty(block: u64) {
		log::debug!("[ADJUST] difficulty on block {}", block);

		let last_base_target = Self::get_last_base_target().0;
		let _last_net_difficulty = Self::get_last_base_target().1;

		let ave_deadline = Self::get_ave_deadline().1;
		let mining_count = Self::get_ave_deadline().0;

		if ave_deadline < 2000 && mining_count > 0 {
			let mut new = last_base_target.saturating_mul(10) / SPEED;
			if new == 0 {
				new = 1;
			}

			log::debug!("[DIFFICULTY] make more difficult, base_target = {:?}", new);
			Self::append_target_info(Difficulty {
				block,
				base_target: new,
				net_difficulty: T::GenesisBaseTarget::get() / new,
			});
		} else if ave_deadline > 3000 {
			let new = last_base_target.saturating_mul(SPEED) / 10;
			Self::append_target_info(Difficulty {
				block,
				base_target: new,
				net_difficulty: T::GenesisBaseTarget::get() / new,
			});
			log::debug!("[DIFFICULTY] make easier,  base_target = {}", new);
		} else {
			let new = last_base_target;
			log::debug!("[DIFFICULTY] use avg,  base_target = {}", new);
			Self::append_target_info(Difficulty {
				block,
				base_target: new,
				net_difficulty: T::GenesisBaseTarget::get() / new,
			});
		}
	}

	fn get_total_capacity() -> u64 {
		let mut old_target_info_vec = TargetInfo::<T>::get();
		let len = old_target_info_vec.len();
		if len > 6 {
			let new_target_info = old_target_info_vec.split_off(len - 6);
			old_target_info_vec = new_target_info;
		}
		let len = old_target_info_vec.len() as u64;

		let mut total_difficulty = 0u64;

		for i in old_target_info_vec.iter() {
			total_difficulty += i.net_difficulty;
		}

		#[allow(unused_assignments)]
		let mut avg_difficulty = 0;
		if len == 0 {
			avg_difficulty = 0;
		} else {
			avg_difficulty = total_difficulty / len;
		}

		let capacity = avg_difficulty.saturating_mul(GIB * CapacityOfPerDifficulty::<T>::get());

		NetPower::<T>::put(capacity);

		return capacity;
	}

	fn reward_staker(miner: T::AccountId, reward: BalanceOf<T>) -> DispatchResult {
		let now = <pallet_poc_staking::Pallet<T>>::now();

		let staking_info = <pallet_poc_staking::Pallet<T>>::staking_info_of(&miner)
			.ok_or(Error::<T>::NotRegister)?;
		let stakers = staking_info.clone().others;
		if stakers.len() == 0 {
			Self::reward_miner(miner.clone(), reward, now);
		} else {
			let miner_reward = staking_info.clone().miner_proportion * reward;
			Self::reward_miner(miner.clone(), miner_reward, now);

			let stakers_reward = reward - miner_reward;
			let total_staking = staking_info.clone().total_staking;
			for staker_info in stakers.iter() {
				let staker_reward = stakers_reward
					.saturating_mul(staker_info.clone().1)
					.checked_div(&total_staking)
					.ok_or(Error::<T>::DivZero)?;

				if staker_info.clone().0 == miner.clone() {
					Self::reward_miner(miner.clone(), staker_reward, now);
				} else {
					T::PocAddOrigin::on_unbalanced(T::StakingCurrency::deposit_creating(
						&staker_info.clone().0,
						staker_reward,
					));
					Self::update_reword_history(staker_info.clone().0, staker_reward, now);
				}
			}
		}

		Ok(())
	}

	fn get_treasury_id() -> T::AccountId {
		<pallet_treasury::Pallet<T>>::account_id()
	}

	fn get_last_base_target() -> (u64, u64) {
		let ti = Self::target_info();

		if let Some(info) = ti.iter().last() {
			(info.base_target, info.net_difficulty)
		} else {
			(T::GenesisBaseTarget::get(), 1u64)
		}
	}

	fn get_ave_deadline() -> (u64, u64) {
		let dl = Self::dl_info();
		let mut iter = dl.iter().rev();
		let mut count = 0_u64;
		let mut real_count = 0_u64;
		let mut deadline = 0_u64;

		while let Some(dl) = iter.next() {
			if count == AdjustDifficultyDuration::<T>::get() / MININGEXPIRE {
				break;
			}
			if dl.miner.is_some() {
				real_count += 1;
				deadline += dl.best_dl;
			}

			count += 1;
		}

		if real_count == 0 {
			(0, 0u64)
		} else {
			(real_count, deadline / real_count)
		}
	}

	fn reward_miner(miner: T::AccountId, amount: BalanceOf<T>, now: T::BlockNumber) {
		let disk = <pallet_poc_staking::Pallet<T>>::disk_of(&miner).unwrap();
		let reward_dest = disk.reward_dest;
		if reward_dest == miner.clone() {
			T::PocAddOrigin::on_unbalanced(T::StakingCurrency::deposit_creating(
				&reward_dest,
				amount,
			));
			Self::update_reword_history(reward_dest.clone(), amount, now);
		} else {
			let miner_reward = Percent::from_percent(10) * amount;
			T::PocAddOrigin::on_unbalanced(T::StakingCurrency::deposit_creating(
				&miner,
				miner_reward,
			));
			Self::update_reword_history(miner, miner_reward, now);

			let dest_reward = amount.saturating_sub(miner_reward);
			T::PocAddOrigin::on_unbalanced(T::StakingCurrency::deposit_creating(
				&reward_dest,
				dest_reward,
			));
			Self::update_reword_history(reward_dest, dest_reward, now);
		}
	}

	fn update_reword_history(
		account_id: T::AccountId,
		amount: BalanceOf<T>,
		block_num: T::BlockNumber,
	) {
		let mut reward_history = UserRewardHistory::<T>::get(account_id.clone()).unwrap();

		reward_history.push((block_num, amount));

		if reward_history.len() >= 300 {
			let mut old_history = reward_history.clone();
			let new_history = old_history.split_off(1);
			<UserRewardHistory<T>>::insert(account_id, new_history);
		} else {
			<UserRewardHistory<T>>::insert(account_id, reward_history);
		}
	}

	fn verify_dl(
		account_id: u64,
		height: u64,
		sig: [u8; 32],
		nonce: u64,
		deadline: u64,
	) -> (bool, u64, u64) {
		let scoop_data = calculate_scoop(height, &sig) as u64;
		log::debug!("scoop_data: {:?}", scoop_data);
		log::debug!("sig: {:?}", sig);

		let mut cache = vec![0_u8; 262144];

		noncegen_rust(&mut cache[..], account_id, nonce, 1);

		let mirror_scoop_data = Self::gen_mirror_scoop_data(scoop_data, cache);

		let (target, _) = find_best_deadline_rust(mirror_scoop_data.as_ref(), 1, &sig);
		log::debug!("target: {:?}", target);
		let base_target = Self::get_current_base_target();
		let deadline_ = target / base_target;
		log::debug!("deadline: {:?}", deadline_);
		(deadline == target / base_target, target, base_target)
	}

	fn gen_mirror_scoop_data(scoop_data: u64, cache: Vec<u8>) -> Vec<u8> {
		let addr = 64 * scoop_data as usize;
		let mirror_scoop = 4095 - scoop_data as usize;
		let mirror_addr = 64 * mirror_scoop as usize;
		let mut mirror_scoop_data = vec![0_u8; 64];
		mirror_scoop_data[0..32].clone_from_slice(&cache[addr..addr + 32]);
		mirror_scoop_data[32..64].clone_from_slice(&cache[mirror_addr + 32..mirror_addr + 64]);
		mirror_scoop_data
	}

	fn get_current_base_target() -> u64 {
		let ti = Self::target_info();
		ti.iter().last().unwrap().base_target
	}
}
