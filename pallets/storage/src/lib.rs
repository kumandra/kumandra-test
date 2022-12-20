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
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
pub use weights::WeightInfo;

use sp_runtime::{traits::SaturatedConversion, RuntimeDebug};

use frame_support::{
	dispatch::DispatchResult,
	traits::{BalanceStatus, Currency, ReservableCurrency},
};

pub const KB: u64 = 1024;

pub const GB: u64 = 1024 * 1024 * 1024;

pub const BASIC_BALANCE: u64 = 100000000000000;

// When whose times of violation is more than 3,
// slash all funds of this miner.
pub const MAX_VIOLATION_TIMES: u64 = 3;

// millisecond * sec * min * hour
// pub const DAY: u64 = 1000 * 60 * 60 * 24;
pub const DAY: u64 = 1000 * 60 * 60 * 24;

// max list order len
pub const NUM_LIST_ORDER_LEN: usize = 500;

// history len
pub const NUM_LIST_HISTORY_LEN: usize = 500;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct Miner<AccountId, Balance> {
	// account id
	pub account_id: AccountId,
	// miner nickname
	pub nickname: Vec<u8>,
	// where miner server locates
	pub region: Vec<u8>,
	// the miner's url
	pub url: Vec<u8>,
	// public_key
	pub public_key: Vec<u8>,
	// stash_address
	pub stash_address: AccountId,
	// capacity of data miner can store
	pub capacity: u128,
	// price per KB every day
	pub unit_price: Balance,
	// times of violations
	pub violation_times: u64,
	// total staking = unit_price * capacity
	pub total_staking: Balance,
	// register time
	pub create_ts: u64,
	// update timestamp
	pub update_ts: u64,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct Order<AccountId, Balance> {
	// miner account id
	pub miner: AccountId,
	// the label of this data
	pub label: Vec<u8>,
	// the hash of data
	pub hash: [u8; 46],
	// the size of storing data(byte)
	pub size: u128,

	pub user: AccountId,

	pub orders: Vec<MinerOrder<AccountId, Balance>>,

	pub status: OrderStatus,
	// register time
	pub create_ts: u64,
	// last update-status timestamp
	pub update_ts: u64,
	// how long this data keep
	pub duration: u64,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct MinerOrder<AccountId, Balance> {
	pub miner: AccountId,
	// one day price = unit_price * data_length
	pub day_price: Balance,
	// total_price = day_price * days
	pub total_price: Balance,
	// last verify result
	pub verify_result: bool,
	// last verify timestamp
	pub verify_ts: u64,
	// confirm order timestamp
	pub confirm_ts: u64,
	// use to be read data
	pub url: Option<Vec<u8>>,
}

/// History
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct MiningHistory<Balance, BlockNumber> {
	// pub miner: AccountId,
	pub total_num: u64,
	pub history: Vec<(BlockNumber, Balance)>,
}

#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum OrderStatus {
	Created,
	Confirmed,
	Expired,
	Deleted,
}

pub type BalanceOf<T> =
	<<T as Config>::StakingCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type StakingCurrency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		type WeightInfo: WeightInfo;
	}

	/// order id is the index of vec.
	#[pallet::storage]
	#[pallet::getter(fn orders)]
	pub(super) type Orders<T: Config> =
		StorageValue<_, Vec<Order<T::AccountId, BalanceOf<T>>>, ValueQuery>;

	/// orders
	#[pallet::storage]
	#[pallet::getter(fn list_order)]
	pub(super) type ListOrder<T: Config> =
		StorageValue<_, Vec<Order<T::AccountId, BalanceOf<T>>>, ValueQuery>;

	/// whose url?.
	#[pallet::storage]
	#[pallet::getter(fn url)]
	pub(super) type Url<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, T::AccountId>;

	/// the info of miners.
	#[pallet::storage]
	#[pallet::getter(fn miners)]
	pub(super) type Miners<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Miner<T::AccountId, BalanceOf<T>>>;

	/// the rewad history of users.
	#[pallet::storage]
	#[pallet::getter(fn history)]
	pub(super) type History<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, MiningHistory<BalanceOf<T>, T::BlockNumber>>;

	/// the rewad history of miners.
	#[pallet::storage]
	#[pallet::getter(fn miner_history)]
	pub(super) type MinerHistory<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<Order<T::AccountId, BalanceOf<T>>>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CreatedOrder { user: T::AccountId },
		Minning { miner: T::AccountId, deadline: u64 },
		VerifyStorage { miner: T::AccountId, verify: bool },
		Registered { who: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// the capacity should not empty.
		CapacityIsZero,
		/// Miners provide insufficient storage capacity
		InsufficientCapacity,
		/// Miner not found.
		MinerNotFound,
		/// None Capacity
		NoneCapacity,
		/// None day
		NoneDays,
		/// url already exists
		UrlExists,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			let current_block = n;
			let n = n.saturated_into::<u64>();
			// Check verifying result per 20 blocks,
			// 20 blocks just 1 minute.
			if n % 20 != 0 {
				return;
			}
			let now = Self::get_now_ts();
			Orders::<T>::mutate(|orders| {
				for mut order in orders {
					if order.status == OrderStatus::Confirmed {
						let create_ts = order.create_ts;
						for mo in &order.orders {
							if now > order.duration + create_ts + DAY {
								order.status = OrderStatus::Expired;
							} else {
								if now - order.update_ts >= DAY && mo.verify_result {
									// verify result is ok, transfer one day's funds to miner
									//  transfer to income address
									if let Some(miner) = Miners::<T>::get(&mo.miner) {
										T::StakingCurrency::repatriate_reserved(
											&order.user,
											&miner.stash_address,
											mo.day_price,
											BalanceStatus::Free,
										)
										.unwrap();

										log::debug!("miner: {:?}", &miner);

										order.update_ts = now;

										Self::update_history(
											current_block,
											mo.miner.clone(),
											mo.day_price,
										);

										Self::deposit_event(Event::<T>::VerifyStorage {
											miner: mo.miner.clone(),
											verify: true,
										});
									}
								} else {
									// verify result expired or no verifying, punish miner
									// Self::punish(&mo.miner, order.size);
									Self::deposit_event(Event::<T>::VerifyStorage {
										miner: mo.miner.clone(),
										verify: false,
									});
								}
							}
						}
					}
				}
			});
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// register
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_miner())]
		pub fn register_miner(
			origin: OriginFor<T>,
			nickname: Vec<u8>,
			region: Vec<u8>,
			url: Vec<u8>,
			public_key: Vec<u8>,
			stash_address: T::AccountId,
			capacity: u128,
			unit_price: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let total_staking = capacity.saturated_into::<BalanceOf<T>>() * unit_price;

			ensure!(capacity > 0, Error::<T>::NoneCapacity);

			ensure!(!Url::<T>::contains_key(url.clone()), Error::<T>::UrlExists);

			Url::<T>::insert(url.clone(), &who);

			Miners::<T>::insert(
				&who,
				Miner {
					account_id: who.clone(),
					nickname,
					region,
					url,
					public_key,
					stash_address,
					capacity,
					unit_price,
					violation_times: 0,
					total_staking,
					create_ts: Self::get_now_ts(),
					update_ts: Self::get_now_ts(),
				},
			);
			Self::deposit_event(Event::<T>::Registered { who });

			Ok(())
		}

		/// the user create the order.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::register_miner())]
		pub fn create_order(
			origin: OriginFor<T>,
			miner: T::AccountId,
			label: Vec<u8>,
			hash: [u8; 46],
			size: u128,
			url: Option<Vec<u8>>,
			days: u64,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;

			let mut order_list = Vec::new();

			let miner_cp = miner.clone();

			ensure!(<Miners<T>>::contains_key(&miner), Error::<T>::MinerNotFound);
			ensure!(days > 0, Error::<T>::NoneDays);

			if let Some(miner_info) = Miners::<T>::get(&miner).as_mut() {
				ensure!(miner_info.capacity > size, Error::<T>::InsufficientCapacity);

				miner_info.capacity = miner_info.capacity - size;

				let day_price = miner_info.unit_price * size.saturated_into::<BalanceOf<T>>();
				let total_price = day_price * days.saturated_into::<BalanceOf<T>>();

				let miner_order = MinerOrder {
					miner: miner_cp.clone(),
					day_price,
					total_price,
					verify_result: true,
					verify_ts: Self::get_now_ts(),
					confirm_ts: Self::get_now_ts(),
					url,
				};
				T::StakingCurrency::reserve(&user, miner_order.total_price)?;
				order_list.push(miner_order);

				Self::append_or_replace_orders(Order {
					miner: miner_cp.clone(),
					label: label.clone(),
					hash: hash.clone(),
					size: size.clone(),
					user: user.clone(),
					orders: order_list.clone(),
					status: OrderStatus::Confirmed,
					create_ts: Self::get_now_ts(),
					update_ts: Self::get_now_ts(),
					duration: days * DAY,
				});

				Self::update_miner_history(
					miner_cp.clone(),
					Order {
						miner: miner_cp.clone(),
						label: label.clone(),
						hash: hash.clone(),
						size: size.clone(),
						user: user.clone(),
						orders: order_list.clone(),
						status: OrderStatus::Confirmed,
						create_ts: Self::get_now_ts(),
						update_ts: Self::get_now_ts(),
						duration: days * DAY,
					},
				);

				Orders::<T>::mutate(|o| {
					o.push(Order {
						miner: miner_cp.clone(),
						label: label.clone(),
						hash: hash.clone(),
						size: size.clone(),
						user: user.clone(),
						orders: order_list,
						status: OrderStatus::Confirmed,
						create_ts: Self::get_now_ts(),
						update_ts: Self::get_now_ts(),
						duration: days * DAY,
					})
				});

				Miners::<T>::insert(&miner_cp.clone(), miner_info);
			}

			Self::deposit_event(Event::<T>::CreatedOrder { user });

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn get_now_ts() -> u64 {
		let now = <pallet_timestamp::Pallet<T>>::get();
		<T::Moment as TryInto<u64>>::try_into(now).ok().unwrap()
	}

	fn update_history(n: T::BlockNumber, miner: T::AccountId, amount: BalanceOf<T>) {
		let mut history = <History<T>>::get(miner.clone());
		if history.is_some() {
			let mut vec = history.clone().unwrap().history;
			let num = history.clone().unwrap().total_num;
			vec.push((n, amount));

			let len = vec.len();
			if len >= 100 {
				let pre = len - 100;
				let new_vec = vec.split_off(pre);
				vec = new_vec;
			}

			history = Some(MiningHistory {
				// miner: miner_cp,
				total_num: num + 1u64,
				history: vec,
			});
		} else {
			let mut vec = vec![];
			vec.push((n, amount));
			history = Some(MiningHistory {
				// miner: miner_cp,
				total_num: 1u64,
				history: vec,
			});
		}

		History::<T>::insert(miner, history.unwrap());
	}

	fn append_or_replace_orders(order: Order<T::AccountId, BalanceOf<T>>) {
		ListOrder::<T>::mutate(|orders| {
			let len = orders.len();
			if len == NUM_LIST_ORDER_LEN {
				let pre = len - NUM_LIST_HISTORY_LEN;
				let new_vec = orders.split_off(pre);
				let _orders = new_vec;
			}
			orders.push(order);
			log::debug!("orders vector: {:?}", orders);
		});
	}

	fn update_miner_history(miner: T::AccountId, order: Order<T::AccountId, BalanceOf<T>>) {
		let mut miner_history = MinerHistory::<T>::get(&miner).unwrap();
		let len = miner_history.len();

		if len >= NUM_LIST_HISTORY_LEN {
			let pre = len - NUM_LIST_HISTORY_LEN;
			let new_vec = miner_history.split_off(pre);
			let _miner_history = new_vec;
		}
		miner_history.push(order);
		MinerHistory::<T>::insert(miner, miner_history);
	}
}
