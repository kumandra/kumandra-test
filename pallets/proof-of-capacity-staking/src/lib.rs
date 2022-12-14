pub mod weights;
pub use weights::WeightInfo;

use frame_support::pallet_prelude::{BoundedVec, ConstU32, DispatchResult};
use frame_support::traits::{Currency, LockableCurrency, ReservableCurrency};
pub use pallet::*;
use pallet_support::primitives::GIB;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{Percent, RuntimeDebug};

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
	pub others: BoundedVec<(AccountId, Balance, Balance), ConstU32<25>>,
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

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Create a new crowdloaning campaign.
		Created { id: u32 },
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
		#[pallet::weight(<T as Config>::WeightInfo::create())]
		pub fn create(
			origin: OriginFor<T>,
			plot_size: GIB,
			numeric_id: u128,
			_miner_proportion: u32,
			reward_dest: Option<T::AccountId>,
		) -> DispatchResult {
			let miner = ensure_signed(origin)?;

			// let miner_proportion = Percent::from_percent(miner_proportion as u8);

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

			let now = frame_system::Pallet::<T>::block_number();

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
			Self::deposit_event(Event::<T>::Created { id: 100 });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn is_register(miner: T::AccountId) -> bool {
		if <DiskOf<T>>::contains_key(&miner) && <StakingInfoOf<T>>::contains_key(&miner) {
			true
		} else {
			false
		}
	}
}
