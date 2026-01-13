#![doc = include_str!("../Locales.md")]
use std::{borrow::Cow, collections::HashMap};

#[cfg(any(feature = "serde", test))]
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "serde", test))]
pub fn get_en() -> Locale<'static> {
    serde_json::de::from_str(include_str!("en.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_en_fuck() -> Locale<'static> {
    serde_json::de::from_str(include_str!("en_fuck.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_de() -> Locale<'static> {
    serde_json::de::from_str(include_str!("de.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_ru() -> Locale<'static> {
    serde_json::de::from_str(include_str!("ru.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_it() -> Locale<'static> {
    serde_json::de::from_str(include_str!("it.json")).unwrap()
}
pub fn get_all() -> impl Iterator<Item = (&'static str, Locale<'static>)> {
    [
        ("en", get_en()),
        ("en_fuck", get_en_fuck()),
        ("de", get_de()),
        ("ru", get_ru()),
        ("it", get_it()),
    ]
    .into_iter()
}

/// This can be used to retroactively add fields, that exist in all versions.
macro_rules! get_field {
    ($t:ty; $($var:ident),*; $field:ident: $ret:ty) => {
        impl<'a> $t {
            pub fn $field(&'a self) -> &'a $ret {
                match self {
                    $(Self::$var(this) => &this.$field),*
                }
            }
        }
    };
}
macro_rules! set_field {
    ($t:ty; $($var:ident),*; $field:ident: $ret:ty) => {
        concat_idents::concat_idents!(set_fn = set_, $field {
            impl<'a> $t {
                pub fn set_fn(&mut self, v: $ret) {
                    match self {
                        $(Self::$var(this) => this.$field = v),*
                    }
                }
            }
        });
    };
}
/// This can be used to retroactively add fields, that may not exist in older versions. (currently unused)
macro_rules! maybe_get_field {
    ($t:ty; $($var_not:ident),*; $($var_do:ident),*; $field:ident: $ret:ty) => {
        impl<'a> $t {
            pub fn $field(&'a self) -> Option<&'a $ret> {
                match self {
                    $(Self::$var_do(this) => Some(&this.$field),)*
                    $(Self::$var_not(_) => None,)*
                }
            }
        }
    };
}
macro_rules! maybe_set_field {
    ($t:ty; $($var_not:ident),*; $($var_do:ident),*; $field:ident: $ret:ty) => {
        concat_idents::concat_idents!(set_fn = set_, $field {
            impl<'a> $t {
                pub fn set_fn(&mut self, v: $ret) -> bool {
                    match self {
                        $(Self::$var_do(this) => {this.$field = v; true})*
                        $(Self::$var_not(_) => false),*
                    }
                }
            }
        });
    };
}

/// Versioned total locale
///
/// Use the getter methods to (maybe) access the fields or setters to (maybe) override them
#[derive(Debug, Clone)]
#[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Locale<'a> {
    V1(v1::Locale<'a>),
    V2(v2::Locale<'a>),
}
get_field!(Locale<'a>; V1, V2; bot_disclaimer: Cow<'a, str> );
set_field!(Locale<'a>; V1, V2; bot_disclaimer: Cow<'a, str> );
impl<'a> Locale<'a> {
    pub fn notes(&'a self) -> Notes<'a> {
        match self {
            Self::V1(this) => Notes::V1(&this.notes),
            Self::V2(this) => Notes::V2(&this.notes),
        }
    }
    pub fn notes_mut(&'a mut self) -> NotesMut<'a> {
        match self {
            Self::V1(this) => NotesMut::V1(&mut this.notes),
            Self::V2(this) => NotesMut::V2(&mut this.notes),
        }
    }
    pub fn format(&'a self) -> Format<'a> {
        match self {
            Self::V1(this) => Format::V1(&this.format),
            Self::V2(this) => Format::V1(&this.format),
        }
    }
    pub fn format_mut(&'a mut self) -> FormatMut<'a> {
        match self {
            Self::V1(this) => FormatMut::V1(&mut this.format),
            Self::V2(this) => FormatMut::V1(&mut this.format),
        }
    }
}
/// Versioned locale just for the notes at the beginning of posts
///
/// Use the getter methods to (maybe) access fields
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Notes<'a> {
    V1(&'a v1::Notes<'a>),
    V2(&'a v2::Notes<'a>),
}
get_field!(Notes<'a>; V1, V2; tower: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; tower_mult: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; digits: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; digits_mult: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; approx: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; approx_mult: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; round: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; round_mult: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; too_big: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; too_big_mult: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; remove: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; tetration: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; no_post: Cow<'a, str>);
get_field!(Notes<'a>; V1, V2; mention: Cow<'a, str>);
maybe_get_field!(Notes<'a>; V1; V2; limit_hit: Cow<'a, str>);
/// Versioned locale for the notes at the beginning of posts
///
/// Use the setter methods to (possibly) override them
#[derive(Debug)]
#[non_exhaustive]
pub enum NotesMut<'a> {
    V1(&'a mut v1::Notes<'a>),
    V2(&'a mut v2::Notes<'a>),
}
set_field!(NotesMut<'a>; V1, V2; tower: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; tower_mult: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; digits: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; digits_mult: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; approx: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; approx_mult: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; round: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; round_mult: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; too_big: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; too_big_mult: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; remove: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; tetration: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; no_post: Cow<'a, str>);
set_field!(NotesMut<'a>; V1, V2; mention: Cow<'a, str>);
maybe_set_field!(NotesMut<'a>; V1; V2; limit_hit: Cow<'a, str>);
/// Versioned locale for the formatting of individual calculations
///
/// Use the getter methods to (maybe) access fields
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Format<'a> {
    V1(&'a v1::Format<'a>),
}
get_field!(Format<'a>; V1; capitalize_calc: bool);
get_field!(Format<'a>; V1; termial: Cow<'a, str>);
get_field!(Format<'a>; V1; factorial: Cow<'a, str>);
get_field!(Format<'a>; V1; uple: Cow<'a, str>);
get_field!(Format<'a>; V1; sub: Cow<'a, str>);
get_field!(Format<'a>; V1; negative: Cow<'a, str>);
get_field!(Format<'a>; V1; num_overrides: HashMap<i32, Cow<'a, str>>);
get_field!(Format<'a>; V1; force_num: bool);
get_field!(Format<'a>; V1; nest: Cow<'a, str>);
get_field!(Format<'a>; V1; rough_number: Cow<'a, str>);
get_field!(Format<'a>; V1; exact: Cow<'a, str>);
get_field!(Format<'a>; V1; rough: Cow<'a, str>);
get_field!(Format<'a>; V1; approx: Cow<'a, str>);
get_field!(Format<'a>; V1; digits: Cow<'a, str>);
get_field!(Format<'a>; V1; order: Cow<'a, str>);
get_field!(Format<'a>; V1; all_that: Cow<'a, str>);
impl<'a> Format<'a> {
    pub fn number_format(&'a self) -> NumFormat<'a> {
        match self {
            Self::V1(this) => NumFormat::V1(&this.number_format),
        }
    }
}
/// Versioned locale for the formatting of individual calculations
///
/// Use the setter methods to (possibly) override them
#[derive(Debug)]
#[non_exhaustive]
pub enum FormatMut<'a> {
    V1(&'a mut v1::Format<'a>),
}
set_field!(FormatMut<'a>; V1; capitalize_calc: bool);
set_field!(FormatMut<'a>; V1; termial: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; factorial: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; uple: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; sub: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; negative: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; num_overrides: HashMap<i32, Cow<'a, str>>);
set_field!(FormatMut<'a>; V1; force_num: bool);
set_field!(FormatMut<'a>; V1; nest: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; rough_number: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; exact: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; rough: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; approx: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; digits: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; order: Cow<'a, str>);
set_field!(FormatMut<'a>; V1; all_that: Cow<'a, str>);
impl<'a> FormatMut<'a> {
    pub fn number_format_mut(&'a mut self) -> NumFormatMut<'a> {
        match self {
            Self::V1(this) => NumFormatMut::V1(&mut this.number_format),
        }
    }
}
/// Versioned locale for how numbers are formatted
///
/// Use the getter methods to (maybe) access fields
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NumFormat<'a> {
    V1(&'a v1::NumFormat),
}
get_field!(NumFormat<'a>; V1; decimal: char);
/// Versioned locale for how numbers are formatted
///
/// Use the setter methods to (possibly) override them
#[derive(Debug)]
#[non_exhaustive]
pub enum NumFormatMut<'a> {
    V1(&'a mut v1::NumFormat),
}
get_field!(NumFormatMut<'a>; V1; decimal: char);

/// v1 of locales
pub mod v1 {
    #[cfg(any(feature = "serde", test))]
    use serde::{Deserialize, Serialize};
    use std::{borrow::Cow, collections::HashMap};

    #[derive(Debug, Clone)]
    #[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
    pub struct Locale<'a> {
        pub bot_disclaimer: Cow<'a, str>,
        pub notes: Notes<'a>,
        pub format: Format<'a>,
    }

    #[derive(Debug, Clone)]
    #[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
    pub struct Notes<'a> {
        pub tower: Cow<'a, str>,
        pub tower_mult: Cow<'a, str>,
        pub digits: Cow<'a, str>,
        pub digits_mult: Cow<'a, str>,
        pub approx: Cow<'a, str>,
        pub approx_mult: Cow<'a, str>,
        pub round: Cow<'a, str>,
        pub round_mult: Cow<'a, str>,
        pub too_big: Cow<'a, str>,
        pub too_big_mult: Cow<'a, str>,
        pub remove: Cow<'a, str>,
        pub tetration: Cow<'a, str>,
        pub no_post: Cow<'a, str>,
        pub mention: Cow<'a, str>,
    }

    #[derive(Debug, Clone)]
    #[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
    pub struct Format<'a> {
        pub capitalize_calc: bool,
        pub termial: Cow<'a, str>,
        pub factorial: Cow<'a, str>,
        pub uple: Cow<'a, str>,
        pub sub: Cow<'a, str>,
        pub negative: Cow<'a, str>,
        pub num_overrides: HashMap<i32, Cow<'a, str>>,
        pub force_num: bool,
        pub nest: Cow<'a, str>,
        pub rough_number: Cow<'a, str>,
        pub exact: Cow<'a, str>,
        pub rough: Cow<'a, str>,
        pub approx: Cow<'a, str>,
        pub digits: Cow<'a, str>,
        pub order: Cow<'a, str>,
        pub all_that: Cow<'a, str>,
        pub number_format: NumFormat,
    }

    #[derive(Debug, Clone)]
    #[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
    pub struct NumFormat {
        pub decimal: char,
    }
}
/// v1 of locales
pub mod v2 {
    #[cfg(any(feature = "serde", test))]
    use serde::{Deserialize, Serialize};
    use std::borrow::Cow;

    #[derive(Debug, Clone)]
    #[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
    pub struct Locale<'a> {
        pub bot_disclaimer: Cow<'a, str>,
        pub notes: Notes<'a>,
        pub format: super::v1::Format<'a>,
    }

    #[derive(Debug, Clone)]
    #[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
    pub struct Notes<'a> {
        pub tower: Cow<'a, str>,
        pub tower_mult: Cow<'a, str>,
        pub digits: Cow<'a, str>,
        pub digits_mult: Cow<'a, str>,
        pub approx: Cow<'a, str>,
        pub approx_mult: Cow<'a, str>,
        pub round: Cow<'a, str>,
        pub round_mult: Cow<'a, str>,
        pub too_big: Cow<'a, str>,
        pub too_big_mult: Cow<'a, str>,
        pub remove: Cow<'a, str>,
        pub tetration: Cow<'a, str>,
        pub no_post: Cow<'a, str>,
        pub mention: Cow<'a, str>,
        pub limit_hit: Cow<'a, str>,
    }
}
