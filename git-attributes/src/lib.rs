#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

use std::path::PathBuf;

use bstr::{BStr, BString};
use compact_str::CompactString;
pub use git_glob as glob;

/// The state an attribute can be in, referencing the value.
///
/// Note that this doesn't contain the name.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub enum StateRef<'a> {
    /// The attribute is listed, or has the special value 'true'
    Set,
    /// The attribute has the special value 'false', or was prefixed with a `-` sign.
    Unset,
    /// The attribute is set to the given value, which followed the `=` sign.
    /// Note that values can be empty.
    #[cfg_attr(feature = "serde1", serde(borrow))]
    Value(&'a BStr),
    /// The attribute isn't mentioned with a given path or is explicitly set to `Unspecified` using the `!` sign.
    Unspecified,
}

/// The state an attribute can be in, owning the value.
///
/// Note that this doesn't contain the name.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub enum State {
    /// The attribute is listed, or has the special value 'true'
    Set,
    /// The attribute has the special value 'false', or was prefixed with a `-` sign.
    Unset,
    /// The attribute is set to the given value, which followed the `=` sign.
    /// Note that values can be empty.
    Value(CompactString), // TODO: use `kstring`, maybe it gets a binary string soon
    /// The attribute isn't mentioned with a given path or is explicitly set to `Unspecified` using the `!` sign.
    Unspecified,
}

/// Holds and owns data that represent one validated attribute
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Name(BString, State);

/// Holds validated attribute data as a reference
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
pub struct NameRef<'a>(&'a BStr, StateRef<'a>);

/// Name an attribute and describe it's assigned state.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub struct Assignment {
    /// The name of the attribute.
    pub name: CompactString,
    /// The state of the attribute.
    pub state: State,
}

/// A grouping of lists of patterns while possibly keeping associated to their base path.
///
/// Pattern lists with base path are queryable relative to that base, otherwise they are relative to the repository root.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Default)]
pub struct MatchGroup<T: match_group::Pattern = Attributes> {
    /// A list of pattern lists, each representing a patterns from a file or specified by hand, in the order they were
    /// specified in.
    ///
    /// During matching, this order is reversed.
    pub patterns: Vec<PatternList<T>>,
}

/// A list of patterns which optionally know where they were loaded from and what their base is.
///
/// Knowing their base which is relative to a source directory, it will ignore all path to match against
/// that don't also start with said base.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Default)]
pub struct PatternList<T: match_group::Pattern> {
    /// Patterns and their associated data in the order they were loaded in or specified,
    /// the line number in its source file or its sequence number (_`(pattern, value, line_number)`_).
    ///
    /// During matching, this order is reversed.
    pub patterns: Vec<PatternMapping<T::Value>>,

    /// The path from which the patterns were read, or `None` if the patterns
    /// don't originate in a file on disk.
    pub source: Option<PathBuf>,

    /// The parent directory of source, or `None` if the patterns are _global_ to match against the repository root.
    /// It's processed to contain slashes only and to end with a trailing slash, and is relative to the repository root.
    pub base: Option<BString>,
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct PatternMapping<T> {
    pub pattern: git_glob::Pattern,
    pub value: T,
    pub sequence_number: usize,
}

mod state {
    use crate::{State, StateRef};
    use bstr::ByteSlice;

    impl<'a> StateRef<'a> {
        pub fn to_owned(self) -> State {
            self.into()
        }
    }

    impl<'a> State {
        pub fn as_ref(&'a self) -> StateRef<'a> {
            match self {
                State::Value(v) => StateRef::Value(v.as_bytes().as_bstr()),
                State::Set => StateRef::Set,
                State::Unset => StateRef::Unset,
                State::Unspecified => StateRef::Unspecified,
            }
        }
    }

    impl<'a> From<StateRef<'a>> for State {
        fn from(s: StateRef<'a>) -> Self {
            match s {
                StateRef::Value(v) => State::Value(v.to_str().expect("no illformed unicode").into()),
                StateRef::Set => State::Set,
                StateRef::Unset => State::Unset,
                StateRef::Unspecified => State::Unspecified,
            }
        }
    }
}

pub mod name {
    use crate::{Name, NameRef, StateRef};
    use bstr::{BStr, BString, ByteSlice};

    impl<'a> NameRef<'a> {
        pub fn name(&self) -> &'a BStr {
            self.0
        }

        pub fn state(&self) -> StateRef<'a> {
            self.1
        }

        pub fn to_owned(self) -> Name {
            self.into()
        }
    }

    impl<'a> From<NameRef<'a>> for Name {
        fn from(v: NameRef<'a>) -> Self {
            Name(v.0.to_owned(), v.1.into())
        }
    }

    impl Name {
        pub fn name(&self) -> &BStr {
            self.0.as_bstr()
        }

        pub fn state(&self) -> StateRef<'_> {
            self.1.as_ref()
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Attribute has non-ascii characters or starts with '-': {attribute}")]
    pub struct Error {
        pub attribute: BString,
    }
}

mod match_group;
pub use match_group::{Attributes, Ignore, Match, Pattern};

pub mod parse;

pub fn parse(buf: &[u8]) -> parse::Lines<'_> {
    parse::Lines::new(buf)
}
