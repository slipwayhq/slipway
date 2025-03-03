use slipway_engine::errors::RigError;
use std::str::FromStr;

const MAXIMUM_PLAYLIST_NAME_LENGTH: usize = 256;
const MAXIMUM_DEVICE_NAME_LENGTH: usize = 256;
const MAXIMUM_RIG_NAME_LENGTH: usize = 256;

slipway_engine::utils::create_validated_string_struct!(pub PlaylistName, Some(slipway_engine::SLIPWAY_ALPHANUMERIC_NAME_REGEX_STR), Some(1), MAXIMUM_PLAYLIST_NAME_LENGTH);
slipway_engine::utils::create_validated_string_struct!(pub DeviceName, Some(slipway_engine::SLIPWAY_ALPHANUMERIC_NAME_REGEX_STR), Some(1), MAXIMUM_DEVICE_NAME_LENGTH);
slipway_engine::utils::create_validated_string_struct!(pub RigName, Some(slipway_engine::SLIPWAY_ALPHANUMERIC_NAME_REGEX_STR), Some(1), MAXIMUM_RIG_NAME_LENGTH);
