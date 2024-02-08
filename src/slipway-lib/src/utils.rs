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
            type Err = SlipwayError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if let Some(min_length) = $min_length {
                    if s.len() < min_length {
                        return Err(SlipwayError::InvalidSlipwayPrimitive(
                            stringify!($name).to_string(),
                            format!(
                                "{} is shorter than the minimum length of {}",
                                stringify!($name),
                                min_length
                            ),
                        ));
                    }
                }

                if s.len() > $max_length {
                    return Err(SlipwayError::InvalidSlipwayPrimitive(
                        stringify!($name).to_string(),
                        format!(
                            "{} is longer than the maximum length of {}",
                            stringify!($name),
                            $max_length
                        ),
                    ));
                }

                if let Some(pattern) = $pattern {
                    let regex = Regex::new(pattern).unwrap();
                    if !regex.is_match(&s) {
                        return Err(SlipwayError::InvalidSlipwayPrimitive(
                            stringify!($name).to_string(),
                            format!("{} does not match the required format", stringify!($name)),
                        ));
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

        #[cfg(test)]
        impl $name {
            pub fn for_test(s: &str) -> Self {
                $name::from_str(s).unwrap()
            }
        }
    };
}

pub(crate) use create_validated_string_struct;
