use std::{collections::HashSet, str::FromStr};

use crate::ComponentHandle;

pub(crate) fn quote(s: &str) -> String {
    format!(r#""{}""#, s)
}

pub(crate) fn ch(handle: &str) -> ComponentHandle {
    ComponentHandle::from_str(handle).unwrap()
}

pub(crate) fn ch_set(handles: Vec<&str>) -> HashSet<ComponentHandle> {
    handles.into_iter().map(ch).collect()
}
