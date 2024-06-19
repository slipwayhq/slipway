macro_rules! create_validated_string_struct {
    ($vis:vis $name:ident, $pattern:expr, $min_length:expr, $max_length:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $vis struct $name(pub String);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = AppError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if let Some(min_length) = $min_length {
                    if s.len() < min_length {
                        return Err(AppError::InvalidSlipwayPrimitive{
                            primitive_type: stringify!($name).to_string(),
                            message: format!(
                                "{} is shorter than the minimum length of {}",
                                stringify!($name),
                                min_length
                            ),
                        });
                    }
                }

                if s.len() > $max_length {
                    return Err(AppError::InvalidSlipwayPrimitive{
                        primitive_type: stringify!($name).to_string(),
                        message: format!(
                            "{} is longer than the maximum length of {}",
                            stringify!($name),
                            $max_length
                        ),
                    });
                }

                if let Some(pattern) = $pattern {
                    let regex = Regex::new(pattern).unwrap();
                    if !regex.is_match(&s) {
                        return Err(AppError::InvalidSlipwayPrimitive{
                            primitive_type: stringify!($name).to_string(),
                            message: format!("{} does not match the required format", stringify!($name)),
                        });
                    }
                }

                Ok($name(s.to_string()))
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                $name::from_str(&s).map_err(serde::de::Error::custom)
            }
        }
    };
}

use std::{collections::HashSet, str::FromStr};

pub(crate) use create_validated_string_struct;

use crate::ComponentHandle;

pub fn ch(handle: &str) -> ComponentHandle {
    ComponentHandle::from_str(handle).unwrap()
}

pub fn ch_vec(handles: Vec<&str>) -> HashSet<ComponentHandle> {
    handles.into_iter().map(ch).collect()
}

pub(crate) trait ExpectWith<T> {
    fn expect_with<F>(self, f: F) -> T
    where
        F: FnOnce() -> String;
}

impl<T> ExpectWith<T> for Option<T> {
    fn expect_with<F>(self, f: F) -> T
    where
        F: FnOnce() -> String,
    {
        match self {
            Some(value) => value,
            None => panic!("{}", f()),
        }
    }
}

impl<T, E> ExpectWith<T> for Result<T, E> {
    fn expect_with<F>(self, f: F) -> T
    where
        F: FnOnce() -> String,
    {
        match self {
            Ok(value) => value,
            Err(_) => panic!("{}", f()),
        }
    }
}
