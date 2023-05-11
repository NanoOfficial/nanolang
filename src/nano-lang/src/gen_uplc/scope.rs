/**
 * @file scope.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-11
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Scope(pub(self) Vec<u64>);

impl From<Vec<u64>> for Scope {
    fn from(value: Vec<u64>) -> Self {
        Self(value)
    }
}

impl Scope {
    pub fn push(&mut self, value: u64) {
        self.0.push(value);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn replace(&mut self, mut replacement: Scope) {
        let common = self.common_ancestor(&replacement);

        self.0.drain(0..common.0.len());

        replacement.0.extend(self.0.iter());
        self.0 = replacement.0;
    }

    pub fn common_ancestor(&self, other: &Self) -> Scope {
        let longest_length = self.0.len().max(other.0.len());

        if *self.0 == *other.0 {
            return self.clone();
        }

        for index in 0..longest_length {
            if self.0.get(index).is_none() {
                return self.clone();
            } else if other.0.get(index).is_none() {
                return other.clone();
            } else if self.0[index] != other.0[index] {
                return Scope(self.0[0..index].to_vec());
            }
        }

        Scope::default()
    }
}