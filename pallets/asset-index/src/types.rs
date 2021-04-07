use frame_support::pallet_prelude::*;
use xcm::v0::MultiLocation;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum AssetAvailability {
    Liquid,
    SAFT,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct IndexAssetData<Balance> {
    units: Balance,
    availability: AssetAvailability,
    location: MultiLocation,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum RedemptionState {
    Initiated,
    Unbonding,
    Transferred,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct AssetWithdrawal<AssetId, Balance> {
    asset: AssetId,
    state: RedemptionState,
    units: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct PendingRedemption<AssetId, Balance, BlockNumber> {
    initiated: BlockNumber,
    assets: Vec<AssetWithdrawal<AssetId, Balance>>,
}
