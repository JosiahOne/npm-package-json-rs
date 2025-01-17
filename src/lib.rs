//! `serde` mappings for npm's `package.json` file format.
//!
//! This does only implement the fields defined [in the official npm documentation](https://docs.npmjs.com/files/package.json).
//! It is common enough that packages define custom entries that are required by
//! various tooling.

#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unsafe_code
)]

use serde::{Deserialize, Serialize, Deserializer};
use serde_json::Value;
use std::{collections::BTreeMap, fmt, fs, io::Read, path::Path, str::FromStr};

use std::collections::BTreeMap as Map;
use std::marker::PhantomData;

use serde::de::{self, Visitor, MapAccess};
use void::Void;

mod error;

pub use crate::error::Error;

/// An ordered map for `bin` entries.
pub type BinSet = BTreeMap<String, String>;
/// An ordered map for `dependencies` entries.
pub type DepsSet = BTreeMap<String, String>;
/// An ordered map for `engines` entries.
pub type EnginesSet = BTreeMap<String, String>;
/// An ordered map for `scripts` entries.
pub type ScriptsSet = BTreeMap<String, String>;

/// The result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// A bug contacting form.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bug {
    /// The email to use for contact.
    pub email: Option<String>,
    /// The url to use to submit bugs.
    pub url: Option<String>,
}

impl Bug {
    /// Creates a new default bug.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new bug with the given values.
    pub fn with(email: Option<String>, url: Option<String>) -> Self {
        Self { email, url }
    }
}

impl FromStr for Bug {
    type Err = Void;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Bug {
            url: Some(s.to_string()),
            email: None
        })
    }
}

/// A person.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Person {
    /// The name of a person.
    pub name: String,
    /// The email of a person.
    pub email: Option<String>,
    /// The homepage of the person.
    pub url: Option<String>,
}

impl Person {
    /// Creates a default person.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a person with a given name.
    pub fn with_name(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    /// Creates a person with the given values.
    pub fn with(name: String, email: Option<String>, url: Option<String>) -> Self {
        Self { name, email, url }
    }
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.name, self.email.as_ref(), self.url.as_ref()) {
            (name, Some(email), None) => write!(f, "{} <{}>", name, email),
            (name, None, Some(url)) => write!(f, "{} ({})", name, url),
            (name, None, None) => write!(f, "{}", name),
            (name, Some(email), Some(url)) => write!(f, "{} <{}> ({})", name, email, url),
        }
    }
}

/// A reference to a person.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PersonReference {
    /// A short reference.
    ///
    /// Short references have a fixed format of `John Doe <john@doe.dev> (https://john.doe.dev)`.
    Short(String),
    /// A full reference.
    ///
    /// This type of reference defines the parts using a struct instead of a
    /// shorthand string format.
    Full(Person),
}

/// A reference to a man page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ManReference {
    /// A single man page reference. Points to one single file.
    Single(String),
    /// Multiple man pages, can contain anything from zero to n.
    Multiple(Vec<String>),
}

/// A repository.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Repository {
    /// The version control system that the repository uses.
    pub r#type: String,
    /// The url to the repository.
    pub url: String,
    /// The directory that the repository is in. Often used for monorepos.
    pub directory: Option<String>,
}

/// A repository reference.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RepositoryReference {
    /// A short reference to the repository. Has to have the syntax that `npm install` allows as well. For more information see [here](https://docs.npmjs.com/files/package.json#repository).
    Short(String),
    /// A full reference.
    Full(Repository),
}

/// The top-level `package.json` structure.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    /// The package name.
    pub name: String,
    /// The package version.
    pub version: String,
    /// The optional package description.
    pub description: Option<String>,
    /// The optional list of keywords.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// The optional package homepage.
    pub homepage: Option<String>,
    /// The optional bug contact form.
    #[serde(deserialize_with = "string_or_struct")]
    pub bugs: Option<Bug>,
    /// The optional package license.
    pub license: Option<String>,
    /// The optional author.
    pub author: Option<PersonReference>,
    /// The optional list of contributors.
    #[serde(default)]
    pub contributors: Vec<PersonReference>,
    /// The optional list of files to include. Each entry defines a regex
    /// pattern.
    #[serde(default)]
    pub files: Vec<String>,
    /// The optional package main entry file.
    pub main: Option<String>,
    /// The optional package browser entry file.
    ///
    /// This is usually defined in libraries that are meant to be consumed by
    /// browsers. These Thoes can refer to objects that are not available inside
    /// a `nodejs` environment (like `window`).
    pub browser: Option<String>,
    /// The optional set of binary definitions.
    #[serde(default)]
    pub bin: BinSet,
    /// The optional list of man page references.
    pub man: Option<ManReference>,
    /// The optional repository reference.
    //#[serde(flatten)]
    pub repository: Option<RepositoryReference>,
    /// The optional list of script entries.
    #[serde(default)]
    pub scripts: ScriptsSet,
    /// The optional list of dependencies.
    #[serde(default)]
    pub dependencies: DepsSet,
    /// The optional list of development dependencies.
    #[serde(default)]
    pub dev_dependencies: DepsSet,
    /// The optional list of peer dependencies.
    #[serde(default)]
    pub peer_dependencies: DepsSet,
    /// The optional list of bundled dependencies.
    #[serde(default)]
    pub bundled_dependencies: DepsSet,
    /// The optional list of optional dependencies.
    #[serde(default)]
    pub optional_dependencies: DepsSet,
    /// The optional list of engine entries.
    #[serde(default)]
    pub engines: EnginesSet,
    /// The package privacy.
    #[serde(default)]
    pub private: bool,
    /// The OS' that the package can run on.
    #[serde(default)]
    pub os: Vec<String>,
    /// The CPU architectures that the package can run on.
    #[serde(default)]
    pub cpu: Vec<String>,
    /// The optional config object.
    pub config: Option<Value>,
    /// Other custom fields that have been defined inside the `package.json`
    /// file.
    #[serde(flatten)]
    pub others: BTreeMap<String, Value>,
}

impl Package {
    /// Creates a new default package.
    pub fn new() -> Self {
        Self::default()
    }

    /// Deserializes a `Package` from a file path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read(path.as_ref())?;
        Self::from_slice(content.as_slice())
    }

    /// Deserializes a `Package` from an IO stream.
    pub fn from_reader<R: Read>(r: R) -> Result<Self> {
        Ok(serde_json::from_reader(r)?)
    }

    /// Deserializes a `Package` from bytes.
    pub fn from_slice(v: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(v)?)
    }
}

impl FromStr for Package {
    type Err = Error;

    /// Deserializes a `Package` from a string.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(serde_json::from_str(s)?)
    }
}

// https://serde.rs/string-or-struct.html
fn string_or_struct<'de, T, D>(deserializer: D) -> std::result::Result::<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Void>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Void>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result::<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> std::result::Result::<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

#[cfg(test)]
mod tests {
    use super::Person;

    #[test]
    fn test_person_display() {
        let person = Person {
            name: "John Doe".to_string(),
            email: Some("john@doe.dev".to_string()),
            url: Some("https://john.doe.dev".to_string()),
        };
        let expected = "John Doe <john@doe.dev> (https://john.doe.dev)";
        assert_eq!(expected, person.to_string());
    }
}
