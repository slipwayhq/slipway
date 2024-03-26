use crate::errors::AppError;
use core::fmt;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

const MAXIMUM_LOADER_ID_LENGTH: usize = 1024;

crate::utils::create_validated_string_struct!(pub LoaderId, None, Some(1), MAXIMUM_LOADER_ID_LENGTH);
