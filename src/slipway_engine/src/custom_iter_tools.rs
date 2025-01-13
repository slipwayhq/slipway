use std::{collections::HashSet, fmt::Display, hash::Hash};

pub(crate) trait CustomIterTools: IntoIterator {
    fn sorted(self) -> std::vec::IntoIter<Self::Item>
    where
        Self::Item: Ord,
        Self: Sized,
    {
        let mut items: Vec<Self::Item> = self.into_iter().collect();
        items.sort();
        items.into_iter()
    }

    fn join(self, separator: &str) -> String
    where
        Self: Sized,
        Self::Item: Display,
    {
        let mut iter = self.into_iter();
        let mut result = String::new();
        if let Some(first) = iter.next() {
            result.push_str(&format!("{first}"));
        }
        for item in iter {
            result.push_str(separator);
            result.push_str(&format!("{item}"));
        }
        result
    }

    fn unique(self) -> Unique<Self::IntoIter>
    where
        Self::Item: Eq + Hash,
        Self: Sized,
    {
        Unique {
            iter: self.into_iter(),
            seen: HashSet::new(),
        }
    }
}

impl<T: IntoIterator> CustomIterTools for T {}

pub struct Unique<I: Iterator> {
    iter: I,
    seen: HashSet<<I as Iterator>::Item>,
}

impl<I> Iterator for Unique<I>
where
    I: Iterator,
    I::Item: Eq + Hash + Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .by_ref()
            .find(|item| self.seen.insert(item.clone()))
    }
}
