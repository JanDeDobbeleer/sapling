// @generated by Thrift for configerator/structs/scm/hg/hgclientconf/hgclient.thrift
// This file is probably not the place you want to edit!

#![recursion_limit = "100000000"]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, unused_crate_dependencies)]

extern crate serde;
pub use self::errors::*;
pub use self::types::*;

/// Thrift type definitions for `hgclient`.
pub mod types {
    #![allow(clippy::redundant_closure)]


    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ::serde_derive::Serialize, ::serde_derive::Deserialize)]
    pub struct TimeShard {
        #[serde(default)]
        pub start: ::std::string::String,
        #[serde(default)]
        pub end: ::std::string::String,
    }

    #[derive(Clone, Debug, PartialEq, ::serde_derive::Serialize, ::serde_derive::Deserialize)]
    pub enum Condition {
        not_condition(::std::boxed::Box<crate::types::Condition>),
        and_condition(::std::vec::Vec<crate::types::Condition>),
        or_condition(::std::vec::Vec<crate::types::Condition>),
        repos(::std::vec::Vec<::std::string::String>),
        platforms(::std::vec::Vec<::std::string::String>),
        domains(::std::vec::Vec<::std::string::String>),
        tiers(::std::vec::Vec<::std::string::String>),
        hosts(::std::vec::Vec<::std::string::String>),
        group(::std::string::String),
        shard(::std::primitive::i32),
        user_shard(::std::primitive::i32),
        time_shard(crate::types::TimeShard),
        host_prefixes(::std::vec::Vec<::std::string::String>),
        UnknownField(::std::primitive::i32),
    }

    #[derive(Clone, Debug, PartialEq, ::serde_derive::Serialize, ::serde_derive::Deserialize)]
    pub struct Hotfix {
        #[serde(default)]
        pub config: ::std::string::String,
        #[serde(default)]
        pub condition: crate::types::Condition,
    }

    #[derive(Clone, Debug, PartialEq, ::serde_derive::Serialize, ::serde_derive::Deserialize)]
    pub struct ClientConfig {
        #[serde(default)]
        pub hotfixes: ::std::vec::Vec<crate::types::Hotfix>,
    }

    impl ::std::default::Default for self::TimeShard {
        fn default() -> Self {
            Self {
                start: ::std::default::Default::default(),
                end: ::std::default::Default::default(),
            }
        }
    }

    unsafe impl ::std::marker::Send for self::TimeShard {}
    unsafe impl ::std::marker::Sync for self::TimeShard {}

    impl ::fbthrift::GetTType for self::TimeShard {
        const TTYPE: ::fbthrift::TType = ::fbthrift::TType::Struct;
    }

    impl<P> ::fbthrift::Serialize<P> for self::TimeShard
    where
        P: ::fbthrift::ProtocolWriter,
    {
        fn write(&self, p: &mut P) {
            p.write_struct_begin("TimeShard");
            p.write_field_begin("start", ::fbthrift::TType::String, 1);
            ::fbthrift::Serialize::write(&self.start, p);
            p.write_field_end();
            p.write_field_begin("end", ::fbthrift::TType::String, 2);
            ::fbthrift::Serialize::write(&self.end, p);
            p.write_field_end();
            p.write_field_stop();
            p.write_struct_end();
        }
    }

    impl<P> ::fbthrift::Deserialize<P> for self::TimeShard
    where
        P: ::fbthrift::ProtocolReader,
    {
        fn read(p: &mut P) -> ::anyhow::Result<Self> {
            static FIELDS: &[::fbthrift::Field] = &[
                ::fbthrift::Field::new("end", ::fbthrift::TType::String, 2),
                ::fbthrift::Field::new("start", ::fbthrift::TType::String, 1),
            ];
            let mut field_start = ::std::option::Option::None;
            let mut field_end = ::std::option::Option::None;
            let _ = p.read_struct_begin(|_| ())?;
            loop {
                let (_, fty, fid) = p.read_field_begin(|_| (), FIELDS)?;
                match (fty, fid as ::std::primitive::i32) {
                    (::fbthrift::TType::Stop, _) => break,
                    (::fbthrift::TType::String, 1) => field_start = ::std::option::Option::Some(::fbthrift::Deserialize::read(p)?),
                    (::fbthrift::TType::String, 2) => field_end = ::std::option::Option::Some(::fbthrift::Deserialize::read(p)?),
                    (fty, _) => p.skip(fty)?,
                }
                p.read_field_end()?;
            }
            p.read_struct_end()?;
            ::std::result::Result::Ok(Self {
                start: field_start.unwrap_or_default(),
                end: field_end.unwrap_or_default(),
            })
        }
    }



    impl ::std::default::Default for Condition {
        fn default() -> Self {
            Self::UnknownField(-1)
        }
    }

    impl ::fbthrift::GetTType for Condition {
        const TTYPE: ::fbthrift::TType = ::fbthrift::TType::Struct;
    }

    impl<P> ::fbthrift::Serialize<P> for Condition
    where
        P: ::fbthrift::ProtocolWriter,
    {
        fn write(&self, p: &mut P) {
            p.write_struct_begin("Condition");
            match self {
                Condition::not_condition(inner) => {
                    p.write_field_begin("not_condition", ::fbthrift::TType::Struct, 1);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::and_condition(inner) => {
                    p.write_field_begin("and_condition", ::fbthrift::TType::List, 2);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::or_condition(inner) => {
                    p.write_field_begin("or_condition", ::fbthrift::TType::List, 3);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::repos(inner) => {
                    p.write_field_begin("repos", ::fbthrift::TType::List, 4);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::platforms(inner) => {
                    p.write_field_begin("platforms", ::fbthrift::TType::List, 5);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::domains(inner) => {
                    p.write_field_begin("domains", ::fbthrift::TType::List, 6);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::tiers(inner) => {
                    p.write_field_begin("tiers", ::fbthrift::TType::List, 7);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::hosts(inner) => {
                    p.write_field_begin("hosts", ::fbthrift::TType::List, 8);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::group(inner) => {
                    p.write_field_begin("group", ::fbthrift::TType::String, 9);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::shard(inner) => {
                    p.write_field_begin("shard", ::fbthrift::TType::I32, 10);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::user_shard(inner) => {
                    p.write_field_begin("user_shard", ::fbthrift::TType::I32, 11);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::time_shard(inner) => {
                    p.write_field_begin("time_shard", ::fbthrift::TType::Struct, 12);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::host_prefixes(inner) => {
                    p.write_field_begin("host_prefixes", ::fbthrift::TType::List, 13);
                    ::fbthrift::Serialize::write(inner, p);
                    p.write_field_end();
                }
                Condition::UnknownField(_) => {}
            }
            p.write_field_stop();
            p.write_struct_end();
        }
    }

    impl<P> ::fbthrift::Deserialize<P> for Condition
    where
        P: ::fbthrift::ProtocolReader,
    {
        fn read(p: &mut P) -> ::anyhow::Result<Self> {
            static FIELDS: &[::fbthrift::Field] = &[
                ::fbthrift::Field::new("and_condition", ::fbthrift::TType::List, 2),
                ::fbthrift::Field::new("domains", ::fbthrift::TType::List, 6),
                ::fbthrift::Field::new("group", ::fbthrift::TType::String, 9),
                ::fbthrift::Field::new("host_prefixes", ::fbthrift::TType::List, 13),
                ::fbthrift::Field::new("hosts", ::fbthrift::TType::List, 8),
                ::fbthrift::Field::new("not_condition", ::fbthrift::TType::Struct, 1),
                ::fbthrift::Field::new("or_condition", ::fbthrift::TType::List, 3),
                ::fbthrift::Field::new("platforms", ::fbthrift::TType::List, 5),
                ::fbthrift::Field::new("repos", ::fbthrift::TType::List, 4),
                ::fbthrift::Field::new("shard", ::fbthrift::TType::I32, 10),
                ::fbthrift::Field::new("tiers", ::fbthrift::TType::List, 7),
                ::fbthrift::Field::new("time_shard", ::fbthrift::TType::Struct, 12),
                ::fbthrift::Field::new("user_shard", ::fbthrift::TType::I32, 11),
            ];
            let _ = p.read_struct_begin(|_| ())?;
            let mut once = false;
            let mut alt = ::std::option::Option::None;
            loop {
                let (_, fty, fid) = p.read_field_begin(|_| (), FIELDS)?;
                match (fty, fid as ::std::primitive::i32, once) {
                    (::fbthrift::TType::Stop, _, _) => break,
                    (::fbthrift::TType::Struct, 1, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::not_condition(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::List, 2, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::and_condition(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::List, 3, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::or_condition(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::List, 4, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::repos(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::List, 5, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::platforms(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::List, 6, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::domains(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::List, 7, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::tiers(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::List, 8, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::hosts(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::String, 9, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::group(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::I32, 10, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::shard(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::I32, 11, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::user_shard(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::Struct, 12, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::time_shard(::fbthrift::Deserialize::read(p)?));
                    }
                    (::fbthrift::TType::List, 13, false) => {
                        once = true;
                        alt = ::std::option::Option::Some(Condition::host_prefixes(::fbthrift::Deserialize::read(p)?));
                    }
                    (fty, _, false) => p.skip(fty)?,
                    (badty, badid, true) => return ::std::result::Result::Err(::std::convert::From::from(::fbthrift::ApplicationException::new(
                        ::fbthrift::ApplicationExceptionErrorCode::ProtocolError,
                        format!(
                            "unwanted extra union {} field ty {:?} id {}",
                            "Condition",
                            badty,
                            badid,
                        ),
                    ))),
                }
                p.read_field_end()?;
            }
            p.read_struct_end()?;
            ::std::result::Result::Ok(alt.unwrap_or_default())
        }
    }

    impl ::std::default::Default for self::Hotfix {
        fn default() -> Self {
            Self {
                config: ::std::default::Default::default(),
                condition: ::std::default::Default::default(),
            }
        }
    }

    unsafe impl ::std::marker::Send for self::Hotfix {}
    unsafe impl ::std::marker::Sync for self::Hotfix {}

    impl ::fbthrift::GetTType for self::Hotfix {
        const TTYPE: ::fbthrift::TType = ::fbthrift::TType::Struct;
    }

    impl<P> ::fbthrift::Serialize<P> for self::Hotfix
    where
        P: ::fbthrift::ProtocolWriter,
    {
        fn write(&self, p: &mut P) {
            p.write_struct_begin("Hotfix");
            p.write_field_begin("config", ::fbthrift::TType::String, 1);
            ::fbthrift::Serialize::write(&self.config, p);
            p.write_field_end();
            p.write_field_begin("condition", ::fbthrift::TType::Struct, 2);
            ::fbthrift::Serialize::write(&self.condition, p);
            p.write_field_end();
            p.write_field_stop();
            p.write_struct_end();
        }
    }

    impl<P> ::fbthrift::Deserialize<P> for self::Hotfix
    where
        P: ::fbthrift::ProtocolReader,
    {
        fn read(p: &mut P) -> ::anyhow::Result<Self> {
            static FIELDS: &[::fbthrift::Field] = &[
                ::fbthrift::Field::new("condition", ::fbthrift::TType::Struct, 2),
                ::fbthrift::Field::new("config", ::fbthrift::TType::String, 1),
            ];
            let mut field_config = ::std::option::Option::None;
            let mut field_condition = ::std::option::Option::None;
            let _ = p.read_struct_begin(|_| ())?;
            loop {
                let (_, fty, fid) = p.read_field_begin(|_| (), FIELDS)?;
                match (fty, fid as ::std::primitive::i32) {
                    (::fbthrift::TType::Stop, _) => break,
                    (::fbthrift::TType::String, 1) => field_config = ::std::option::Option::Some(::fbthrift::Deserialize::read(p)?),
                    (::fbthrift::TType::Struct, 2) => field_condition = ::std::option::Option::Some(::fbthrift::Deserialize::read(p)?),
                    (fty, _) => p.skip(fty)?,
                }
                p.read_field_end()?;
            }
            p.read_struct_end()?;
            ::std::result::Result::Ok(Self {
                config: field_config.unwrap_or_default(),
                condition: field_condition.unwrap_or_default(),
            })
        }
    }


    impl ::std::default::Default for self::ClientConfig {
        fn default() -> Self {
            Self {
                hotfixes: ::std::default::Default::default(),
            }
        }
    }

    unsafe impl ::std::marker::Send for self::ClientConfig {}
    unsafe impl ::std::marker::Sync for self::ClientConfig {}

    impl ::fbthrift::GetTType for self::ClientConfig {
        const TTYPE: ::fbthrift::TType = ::fbthrift::TType::Struct;
    }

    impl<P> ::fbthrift::Serialize<P> for self::ClientConfig
    where
        P: ::fbthrift::ProtocolWriter,
    {
        fn write(&self, p: &mut P) {
            p.write_struct_begin("ClientConfig");
            p.write_field_begin("hotfixes", ::fbthrift::TType::List, 1);
            ::fbthrift::Serialize::write(&self.hotfixes, p);
            p.write_field_end();
            p.write_field_stop();
            p.write_struct_end();
        }
    }

    impl<P> ::fbthrift::Deserialize<P> for self::ClientConfig
    where
        P: ::fbthrift::ProtocolReader,
    {
        fn read(p: &mut P) -> ::anyhow::Result<Self> {
            static FIELDS: &[::fbthrift::Field] = &[
                ::fbthrift::Field::new("hotfixes", ::fbthrift::TType::List, 1),
            ];
            let mut field_hotfixes = ::std::option::Option::None;
            let _ = p.read_struct_begin(|_| ())?;
            loop {
                let (_, fty, fid) = p.read_field_begin(|_| (), FIELDS)?;
                match (fty, fid as ::std::primitive::i32) {
                    (::fbthrift::TType::Stop, _) => break,
                    (::fbthrift::TType::List, 1) => field_hotfixes = ::std::option::Option::Some(::fbthrift::Deserialize::read(p)?),
                    (fty, _) => p.skip(fty)?,
                }
                p.read_field_end()?;
            }
            p.read_struct_end()?;
            ::std::result::Result::Ok(Self {
                hotfixes: field_hotfixes.unwrap_or_default(),
            })
        }
    }

}

/// Error return types.
pub mod errors {
}
