use crate::{lua::AsLua, prelude::*};

pub fn do_try<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    f()
}

pub type LuaDict<V> = LuaMap<LuaString, V>;
pub type LuaDictMut<V> = LuaMapMut<LuaString, V>;

#[doc(hidden)]
pub mod __mac {
    use crate::prelude::*;

    pub fn single_key_val<K: PushLua, V: PushLua>(
        key: K,
        val: V,
    ) -> crate::lua::LuaDefer<impl FnOnce(&Lua) -> Result<LuaMapOwned<K, V>>> {
        lua_defer_val(|lua| {
            lua.create_table_from([(key, val)])
                .map(LuaMapOwned::cast_table_any)
        })
    }

    pub fn tbl_seq_new(lua: &Lua) -> Result<LuaSeqOwned<LuaBottom>> {
        LuaSeqOwned::new(lua)
    }
    pub fn tbl_seq_append<T: PushLua, V>(
        seq: LuaSeqOwned<V>,
        item: T,
    ) -> Result<LuaSeqOwned<LuaUnion<T, V>>> {
        let seq = seq.into_table_any();
        seq.push_any(item)?;
        Ok(LuaSeqOwned::cast_table_any(seq))
    }

    pub const fn norm_raw_ident(s: &str) -> &str {
        let Some((pre, rest)) = s.split_at_checked(2) else {
            return s;
        };
        match pre.as_bytes() {
            b"r#" => rest,
            _ => s,
        }
    }
    macro_rules! field_name {
        (#[name = $name:expr] $field:ident) => {
            $name
        };
        ($field:ident) => {{ const { crate::utils::__mac::norm_raw_ident(stringify!($field)) } }};
    }
    pub(crate) use field_name;

    const _: () = {
        assert!(matches!(field_name!(r#match).as_bytes(), b"match"));
        assert!(matches!(field_name!(test).as_bytes(), b"test"));
        assert!(matches!(
            field_name!(
                #[name = "match"]
                match_
            )
            .as_bytes(),
            b"match"
        ));
    };

    #[cfg(feature = "strict-typeck")]
    pub mod nominals {
        use crate::typing::logic::*;

        const fn hash_into(out: &mut u32, mut data: &[u8]) {
            while let &[b, ref rest @ ..] = data {
                data = rest;
                *out ^= b as u32;
                *out = out.wrapping_mul(0x01000193);
            }
        }
        pub const fn hash_all(mut data: &[&[u8]]) -> IdTy {
            let mut hash: u32 = 0x811c9dc5;

            while let [next, rest @ ..] = data {
                data = rest;
                hash_into(&mut hash, next);
            }

            (hash >> 16) as u16 ^ hash as u16
        }

        pub type IdTy = u16;
        pub const fn get_bit(hash: IdTy, i: u16) -> bool {
            (hash >> i) & 1 != 0
        }

        pub trait GetBoolImpl<const B: bool> {
            type Val: Bool;
        }
        impl GetBoolImpl<true> for () {
            type Val = True;
        }
        impl GetBoolImpl<false> for () {
            type Val = False;
        }
        pub type GetBool<const B: bool> = <() as GetBoolImpl<B>>::Val;

        pub trait UniqueIdImpl<const ID: IdTy> {
            type Id: BitSeq;
        }
        pub type UniqueId<const ID: IdTy> = <() as UniqueIdImpl<ID>>::Id;
    }

    #[cfg(feature = "strict-typeck")]
    macro_rules! nominal_id {
        ($name:ident) => {
            crate::utils::__mac::nominals::UniqueId::<{
                use crate::utils::__mac::nominals::*;

                const ID: IdTy = hash_all(&[
                    stringify!($name).as_bytes(),
                    module_path!().as_bytes(),
                    &u32::to_le_bytes(line!()),
                    &u32::to_le_bytes(column!()),
                ]);

                #[allow(non_local_definitions)]
                impl UniqueIdImpl<ID> for () {
                    type Id = (((((((((((((((((),
                        GetBool<{ get_bit(ID, 0) }>),
                        GetBool<{ get_bit(ID, 1) }>),
                        GetBool<{ get_bit(ID, 2) }>),
                        GetBool<{ get_bit(ID, 3) }>),
                        GetBool<{ get_bit(ID, 4) }>),
                        GetBool<{ get_bit(ID, 5) }>),
                        GetBool<{ get_bit(ID, 6) }>),
                        GetBool<{ get_bit(ID, 7) }>),
                        GetBool<{ get_bit(ID, 8) }>),
                        GetBool<{ get_bit(ID, 9) }>),
                        GetBool<{ get_bit(ID, 10) }>),
                        GetBool<{ get_bit(ID, 11) }>),
                        GetBool<{ get_bit(ID, 12) }>),
                        GetBool<{ get_bit(ID, 13) }>),
                        GetBool<{ get_bit(ID, 14) }>),
                        GetBool<{ get_bit(ID, 15) }>);
                }

                ID
            }>
        };
    }
    #[cfg(not(feature = "strict-typeck"))]
    macro_rules! nominal_id {
        ($($t:tt)*) => {
            ()
        };
    }
    pub(crate) use nominal_id;
}

macro_rules! tbl {
    (eval($lua:expr), $($rest:tt)*) => {
        crate::utils::tbl! { $($rest)* }.eval($lua)
    };
    (owned, {$( $t:tt )*}) => {
        crate::utils::tbl! { owned!(crate::lua::LuaString, crate::lua::LuaVal), { $($t)* } }
    };
    (owned!($vt:ty), {$( $t:tt )*}) => {
        crate::utils::tbl! { owned!(crate::lua::LuaString, $vt), { $($t)* } }
    };
    (owned!($kt:ty, $vt:ty), {$( $t:tt )*}) => {
        crate::lua::lua_defer_val(|lua| {
            let out = crate::lua::LuaMapOwned::<$kt, $vt>::new(lua)?;
            crate::utils::tbl! { out(out), { $($t)* } }
        })
    };
    (out($table:expr), {$( $t:tt )*}) => {{
        let out = $table;
        crate::utils::do_try(|| {
            #[allow(unused_imports)]
            use crate::lua::LuaTableSet as _;
            crate::utils::tbl! { @visit out $($t)* }
            Ok(out)
        })
    }};
    (@visit $out:ident $key_or_field:tt = $val:expr; $($t:tt)*) => {
        $out.set(
            crate::utils::tbl!(@key_or_field $key_or_field),
            $val,
        )?;
        crate::utils::tbl! { @visit $out $($t)* }
    };
    (@visit $out:ident $key_or_field:tt$(.$field:ident)+ = $val:expr; $($t:tt)*) => {{
        $out.set(
            crate::utils::tbl!(@key_or_field $key_or_field),
            crate::utils::tbl!(@nest_single_field [$(crate::utils::__mac::field_name!($field)),*], $val),
        )?;
        crate::utils::tbl! { @visit $out $($t)* }
    }};
    (@visit $out:ident) => { () };
    (@visit $($t:tt)*) => {
        compile_error! {
            concat!(
                "Unexpected input:\n",
                crate::utils::__mac::field_name!($($t)*),
            )
        }
    };
    (@key_or_field $field:ident) => { crate::utils::__mac::field_name!($field) };
    (@key_or_field $field:literal) => { $field };
    (@key_or_field [$key:expr]) => { $key };
    (@nest_single_field [$first_field:expr $(,$field:expr)*], $val:expr) => {
        crate::utils::__mac::single_key_val(
            $first_field,
            crate::utils::tbl!(@nest_single_field [$($field),*], $val),
        )
    };
    (@nest_single_field [], $val:expr) => { $val };
}
pub(crate) use tbl;

macro_rules! tbl_seq {
    [$($val:expr),* $(,)?] => {
        crate::lua::lua_defer_val(|lua| {
            let seq = crate::utils::__mac::tbl_seq_new(lua)?;
            $(let seq = crate::utils::__mac::tbl_seq_append(seq, $val)?;)*
            Ok(seq)
        })
    };
}
pub(crate) use tbl_seq;

pub trait ResultExt {
    type Ok;
    type Err;

    fn ok_or_notify(self, lua: impl AsLua) -> Option<Self::Ok>
    where
        Self::Err: Into<Error>;
}
impl<T, E> ResultExt for std::result::Result<T, E> {
    type Ok = T;
    type Err = E;

    fn ok_or_notify(self, lua: impl AsLua) -> Option<T>
    where
        E: Into<Error>,
    {
        match self {
            Ok(ok) => Some(ok),
            Err(err) => {
                crate::env::lua_notify_err(Some(lua.lua()), err.into());
                None
            }
        }
    }
}

macro_rules! builder_struct {
    ({
        $(#[$meta:meta])*
        struct $gname:ident {$(
            $field:ident: $fty:ty
        ),* $(,)? }
    }) => {
        $(#[$meta])*
        #[derive(Default, Debug)]
        #[allow(non_camel_case_types)]
        pub struct $gname<$($field = $fty),*> {
            $(pub $field: $field),*
        }
        #[allow(non_camel_case_types)]
        const _: () = {
            $(type $field = crate::lua::LuaNil;)*
            impl $gname<$($field),*> {
                pub fn _new() -> Self {
                    Self {
                        $($field: crate::lua::LuaNil),*
                    }
                }
            }
        };

        #[allow(non_camel_case_types)]
        const _: () = {
            impl<$($field),*> $gname<$($field),*> {
                crate::utils::builder_struct! { @impl_generic [$($field ($fty))*] $gname [] }
            }
            impl<$($field: crate::lua::PushLua),*> mlua::IntoLua for $gname<$($field),*> {
                fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
                    let table = lua.create_table()?;
                    let Self { $($field),* } = self;
                    $({
                        let val = crate::lua::PushLua::into_mlua($field)?;
                        table.raw_set(crate::utils::__mac::field_name!($field), val)?;
                    })*
                    Ok(mlua::Value::Table(table))
                }
            }
            impl<$($field: crate::lua::PushLua),*> crate::lua::PushLua for $gname<$($field),*> {
                type IntoRepr = Self;
                fn into_mlua(self) -> crate::lua::Result<Self::IntoRepr> {
                    Ok(self)
                }
            }
            impl crate::lua::PopLua for $gname {
                type FromRepr = crate::lua::LuaBottom;
                fn from_mlua(repr: Self::FromRepr) -> crate::lua::Result<Self> {
                    match repr {}
                }
            }
            impl<$($field: crate::typing::LuaSub<$fty>),*> crate::lua::LuaStructInner for $gname<$($field),*> {
                type Fields = ($($fty,)*);
                type NominalId = crate::utils::__mac::nominal_id!($gname);
            }
            impl<$($field: crate::typing::LuaSub<$fty>),*> $gname<$($field),*> {
                pub fn _finish(self) -> crate::lua::LuaStruct::<Self> {
                    crate::lua::LuaStruct(self)
                }
            }
        };
    };
    ( @impl_generic [] $($rest:tt)* ) => {};
    (
        @impl_generic
        [$field:ident ($fty:ty) $($rfield:ident $rfty:tt)*]
        $struct:ident
        [$($lfield:ident)*]
    ) => {
        pub fn $field<_Param>(self, $field: _Param) -> $struct<$($lfield,)* _Param, $($rfield,)*>
        where
            _Param: crate::typing::LuaSub<$fty>,
        {
            let Self { $($lfield,)* $field: _, $($rfield,)* } = self;
            $struct {
                $($lfield,)*
                $field,
                $($rfield,)*
            }
        }
        crate::utils::builder_struct! { @impl_generic [$($rfield $rfty)*] $struct [$($lfield)* $field] }
    }
}
pub(crate) use builder_struct;

macro_rules! mk_builder {
    ($($name:ident)::+, {
        $($field:ident = $val:expr;)*
    }) => {{
        $($name)::+::_new()
            $(.$field($val))*
            ._finish()
    }};
}
pub(crate) use mk_builder;

macro_rules! from_tbl_struct {
    ({
        $(#[$meta:meta])*
        struct $name:ident {$(
            $(#[$fmeta:meta])*
            $field:ident: $fieldty:ty
        ),* $(,)?}
    }) => {
        #[derive(Clone, Debug)]
        $(#[$meta])*
        pub struct $name { pub table: crate::lua::LuaTableAny }
        #[allow(non_snake_case)]
        impl crate::lua::LuaStruct<$name> {$(
            $(#[$fmeta])*
            pub fn $field(&self) -> crate::lua::Result<$fieldty> {
                self.0.table.get_any(crate::utils::__mac::field_name!($field))
            }
        )*}
        impl crate::lua::PopLua for $name {
            type FromRepr = mlua::Table;
            fn from_mlua(repr: Self::FromRepr) -> crate::lua::Result<Self> {
                Ok(Self {
                    table: crate::lua::PopLua::from_mlua(repr)?,
                })
            }
        }
        impl crate::lua::PushLua for $name {
            type IntoRepr = mlua::Table;
            fn into_mlua(self) -> crate::lua::Result<Self::IntoRepr> {
                crate::lua::PushLua::into_mlua(self.table)
            }
        }
        impl crate::lua::LuaStructInner for $name {
            type Fields = ($($fieldty,)*);
            type NominalId = crate::utils::__mac::nominal_id!($name);
        }
    };
}
pub(crate) use from_tbl_struct;

macro_rules! from_tbl_proxy {
    ({
        $(#[$meta:meta])*
        struct $name:ident {$(
            $(#[$fmeta:meta])*
            $field:ident: $fieldty:ty
        ),* $(,)?}
    }) => {
        #[derive(Clone, Debug)]
        $(#[$meta])*
        pub struct $name { pub table: crate::lua::LuaTableAny }
        impl crate::lua::PopLua for $name {
            type FromRepr = mlua::Table;
            fn from_mlua(repr: Self::FromRepr) -> crate::lua::Result<Self> {
                Ok(Self {
                    table: crate::lua::PopLua::from_mlua(repr)?,
                })
            }
        }
        #[allow(non_snake_case)]
        impl $name {$(
            $(#[$fmeta])*
            pub fn $field(&self) -> crate::lua::Result<$fieldty> {
                self.table.get_any(crate::utils::__mac::field_name!($field))
            }
        )*}
    };
}
pub(crate) use from_tbl_proxy;
