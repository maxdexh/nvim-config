use crate::prelude::*;
use std::marker::PhantomData;

pub type Error = crate::error::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait PushLua: Sized {
    type IntoRepr: mlua::IntoLua;
    fn into_mlua(self) -> Result<Self::IntoRepr>;
}
pub trait PopLua: Sized {
    type FromRepr: mlua::FromLua;
    fn from_mlua(repr: Self::FromRepr) -> Result<Self>;
}
pub trait PushLuaMulti: Sized {
    type IntoReprMulti: mlua::IntoLuaMulti;
    fn into_mlua_multi(self) -> Result<Self::IntoReprMulti>;
}
impl<T: PushLua> PushLuaMulti for T {
    type IntoReprMulti = T::IntoRepr;
    fn into_mlua_multi(self) -> Result<Self::IntoReprMulti> {
        self.into_mlua()
    }
}
pub trait PopLuaMulti: Sized {
    type FromReprMulti: mlua::FromLuaMulti;
    fn from_mlua_multi(repr: Self::FromReprMulti) -> Result<Self>;
}
impl<T: PopLua> PopLuaMulti for T {
    type FromReprMulti = T::FromRepr;
    fn from_mlua_multi(repr: Self::FromReprMulti) -> Result<Self> {
        T::from_mlua(repr)
    }
}
macro_rules! impl_tuple {
    () => {};
    ($Tail:ident $(,$T:ident)* $(,)?) => {
        impl_tuple!($($T),*);
        impl<$($T: PopLua,)* $Tail:PopLuaMulti> PopLuaMulti for ($($T,)* $Tail,) {
            type FromReprMulti = ($($T::FromRepr,)* $Tail::FromReprMulti,);
            fn from_mlua_multi(repr: Self::FromReprMulti) -> Result<Self> {
                #[allow(non_snake_case)]
                let ($($T,)* $Tail,) = repr;
                Ok((
                    $($T::from_mlua($T)?,)*
                    $Tail::from_mlua_multi($Tail)?,
                ))
            }
        }
        impl<$($T: PushLua,)* $Tail:PushLuaMulti> PushLuaMulti for ($($T,)* $Tail,) {
            type IntoReprMulti = ($($T::IntoRepr,)* $Tail::IntoReprMulti,);
            fn into_mlua_multi(self) -> Result<Self::IntoReprMulti> {
                #[allow(non_snake_case)]
                let ($($T,)* $Tail,) = self;
                Ok((
                    $($T::into_mlua($T)?,)*
                    $Tail::into_mlua_multi($Tail)?,
                ))
            }
        }
    };
}
impl_tuple!(
    TP, TO, TN, TM, TL, TK, TJ, TI, TH, TG, TF, TE, TD, TC, TB, TA,
);
pub type LuaVariadic<T> = mlua::Variadic<T>;
impl<T: PopLua> PopLuaMulti for LuaVariadic<T> {
    type FromReprMulti = mlua::Variadic<T::FromRepr>;
    fn from_mlua_multi(repr: Self::FromReprMulti) -> Result<Self> {
        // NOTE: unnecessary realloc
        repr.into_iter().map(T::from_mlua).collect()
    }
}
impl<T: PushLua> PushLuaMulti for LuaVariadic<T> {
    type IntoReprMulti = mlua::Variadic<T::IntoRepr>;
    fn into_mlua_multi(self) -> Result<Self::IntoReprMulti> {
        // NOTE: unnecessary realloc
        self.into_iter().map(T::into_mlua).collect()
    }
}

impl<T: PushLua> PushLua for Result<T> {
    type IntoRepr = T::IntoRepr;
    fn into_mlua(self) -> Result<Self::IntoRepr> {
        self.and_then(|it| it.into_mlua())
    }
}
impl<T: PushLua> PushLua for Option<T> {
    type IntoRepr = Option<T::IntoRepr>;
    fn into_mlua(self) -> crate::prelude::Result<Self::IntoRepr> {
        self.map(|it| it.into_mlua()).transpose()
    }
}
impl<T: PopLua> PopLua for Option<T> {
    type FromRepr = Option<T::FromRepr>;
    fn from_mlua(repr: Self::FromRepr) -> Result<Self> {
        repr.map(|it| T::from_mlua(it)).transpose()
    }
}
macro_rules! triv_conv_impl {
    ($tr:ident, $assoc:ident, $func:ident, $self:ident, $t:ty) => {
        impl crate::lua::$tr for $t {
            type $assoc = Self;
            fn $func($self: Self) -> crate::lua::Result<Self> {
                Ok($self)
            }
        }
    };
}
triv_conv_impl!(PushLuaMulti, IntoReprMulti, into_mlua_multi, self, ());
triv_conv_impl!(PopLuaMulti, FromReprMulti, from_mlua_multi, repr, ());
macro_rules! triv_conv_impl_into {
    ($($t:tt)*) => {
        triv_conv_impl!(PushLua, IntoRepr, into_mlua, self, $($t)*);
    };
}
macro_rules! triv_conv_impl_from {
    ($($t:tt)*) => {
        triv_conv_impl!(PopLua, FromRepr, from_mlua, repr, $($t)*);
    };
}
macro_rules! triv_conv_impl_both {
    ($($t:tt)*) => {
        triv_conv_impl_into!($($t)*);
        triv_conv_impl_from!($($t)*);
    };
}
triv_conv_impl_both!(bool);

pub type LuaVal = mlua::Value;
triv_conv_impl_both!(LuaVal);

pub type LuaString = mlua::LuaString;
triv_conv_impl_both!(crate::lua::LuaString);
const _: () = {
    macro_rules! strs {
        ($($int:ty),* $(,)?) => {$(
            triv_conv_impl_into!($int);
        )*};
    }
    strs!(&str, String, &crate::lua::LuaString);
};

pub type LuaNum = mlua::Number;
triv_conv_impl_both!(f32);
triv_conv_impl_both!(f64);

pub type LuaInt = mlua::Integer;
const _: () = {
    macro_rules! ints {
        ($($int:ty),* $(,)?) => {$(
            triv_conv_impl_both!($int);
        )*};
    }
    ints![i8, u8, i16, u16, i32, u32, i64, u64, isize, usize];
};
impl<T: PushLua, const N: usize> PushLua for [T; N] {
    type IntoRepr = [T::IntoRepr; N];
    fn into_mlua(self) -> Result<Self::IntoRepr> {
        let arr = self.map(T::into_mlua);
        if arr.iter().any(|it| it.is_err()) {
            std::hint::cold_path();
            for x in arr {
                x?;
            }
            unreachable!();
        }
        Ok(arr.map(|it| it.unwrap()))
    }
}

pub type LuaUnion<L, R> = mlua::Either<L, R>;
impl<L: PushLua, R: PushLua> PushLua for LuaUnion<L, R> {
    type IntoRepr = LuaUnion<L::IntoRepr, R::IntoRepr>;
    fn into_mlua(self) -> Result<Self::IntoRepr> {
        Ok(match self {
            Self::Left(l) => LuaUnion::Left(l.into_mlua()?),
            Self::Right(r) => LuaUnion::Right(r.into_mlua()?),
        })
    }
}
impl<L: PopLua, R: PopLua> PopLua for LuaUnion<L, R> {
    type FromRepr = LuaUnion<L::FromRepr, R::FromRepr>;
    fn from_mlua(repr: Self::FromRepr) -> Result<Self> {
        Ok(match repr {
            LuaUnion::Left(l) => Self::Left(L::from_mlua(l)?),
            LuaUnion::Right(r) => Self::Right(R::from_mlua(r)?),
        })
    }
}

struct TranslateMlua<T>(T);
impl<T: PushLua> mlua::IntoLua for TranslateMlua<T> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        mlua::IntoLua::into_lua(self.0.into_mlua()?, lua)
    }
}

#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Lua(mlua::Lua);
impl Lua {
    pub fn as_mlua(&self) -> &mlua::Lua {
        &self.0
    }
    fn by_mlua(lua: &mlua::Lua) -> &Self {
        // SAFETY: repr(transparent)
        unsafe { &*std::ptr::from_ref(lua).cast() }
    }
    pub fn from_mlua(lua: mlua::Lua) -> Self {
        Self(lua)
    }
    pub fn create_string(&self, s: impl AsRef<[u8]>) -> Result<LuaString> {
        self.as_mlua().create_string(s).map_err(Into::into)
    }
    pub fn create_table(&self) -> Result<LuaTableAny> {
        Ok(LuaTableAny(self.as_mlua().create_table()?))
    }
    pub fn create_sequence_from<T: PushLua>(
        &self,
        iter: impl IntoIterator<Item = T>,
    ) -> Result<LuaTableAny> {
        self.as_mlua()
            .create_sequence_from(iter.into_iter().map(TranslateMlua))
            .map(LuaTableAny)
            .map_err(Into::into)
    }
    pub fn create_table_from<K: PushLua, V: PushLua>(
        &self,
        iter: impl IntoIterator<Item = (K, V)>,
    ) -> Result<LuaTableAny> {
        self.as_mlua()
            .create_table_from(
                iter.into_iter()
                    .map(|(k, v)| (TranslateMlua(k), TranslateMlua(v))),
            )
            .map(LuaTableAny)
            .map_err(Into::into)
    }
    pub fn globals(&self) -> LuaTableAny {
        LuaTableAny(self.as_mlua().globals())
    }
}

#[derive(Clone, Copy, Default)]
pub struct LuaNil;
impl mlua::IntoLua for LuaNil {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Nil)
    }
}
impl mlua::FromLua for LuaNil {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Nil => Ok(Self),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: std::any::type_name::<Self>().into(),
                message: None,
            }),
        }
    }
}
triv_conv_impl_both!(crate::lua::LuaNil);

#[derive(Debug)]
pub enum LuaBottom {}
impl mlua::IntoLua for LuaBottom {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        match self {}
    }
}
impl mlua::FromLua for LuaBottom {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        Err(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: std::any::type_name::<Self>().into(),
            message: None,
        })
    }
}
triv_conv_impl_both!(crate::lua::LuaBottom);

#[derive(Clone, Debug)]
pub struct LuaTableAny(mlua::Table);
impl LuaTableAny {
    pub fn get_any<R: PopLua>(&self, key: impl PushLua) -> Result<R> {
        self.0
            .get(key.into_mlua()?)
            .map_err(Into::into)
            .and_then(R::from_mlua)
    }
    pub fn set_any(&self, key: impl PushLua, val: impl PushLua) -> Result<()> {
        self.0
            .set(key.into_mlua()?, val.into_mlua()?)
            .map_err(Into::into)
    }
    pub fn push_any(&self, val: impl PushLua) -> Result<()> {
        self.0.push(val.into_mlua()?).map_err(Into::into)
    }
    pub fn raw_push_any(&self, val: impl PushLua) -> Result<()> {
        self.0.raw_push(val.into_mlua()?).map_err(Into::into)
    }
    pub fn sequence_values<T: PopLua>(&self) -> impl IntoIterator<Item = Result<T>> {
        self.0
            .sequence_values()
            .map(|it| it.map_err(Into::into).and_then(T::from_mlua))
    }
}
impl PushLua for LuaTableAny {
    type IntoRepr = mlua::Table;
    fn into_mlua(self) -> Result<Self::IntoRepr> {
        Ok(self.0)
    }
}
impl PopLua for LuaTableAny {
    type FromRepr = mlua::Table;
    fn from_mlua(repr: Self::FromRepr) -> Result<Self> {
        Ok(Self(repr))
    }
}

#[derive(Clone, Debug)]
pub enum LuaCallableAny {
    Func(mlua::Function),
    Data(mlua::AnyUserData),
    Table(mlua::Table),
}
impl LuaCallableAny {
    pub fn call_any<R: PopLuaMulti>(&self, args: impl PushLuaMulti) -> Result<R> {
        use mlua::ObjectLike as _;
        match self {
            Self::Func(func) => func.call(args.into_mlua_multi()?),
            Self::Data(data) => data.call(args.into_mlua_multi()?),
            Self::Table(table) => table.call(args.into_mlua_multi()?),
        }
        .map_err(Into::into)
        .and_then(R::from_mlua_multi)
    }
}
impl mlua::IntoLua for LuaCallableAny {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(match self {
            Self::Func(func) => mlua::Value::Function(func),
            Self::Data(data) => mlua::Value::UserData(data),
            Self::Table(table) => mlua::Value::Table(table),
        })
    }
}
impl mlua::FromLua for LuaCallableAny {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        Ok(match value {
            mlua::Value::Table(table) => Self::Table(table),
            mlua::Value::Function(func) => Self::Func(func),
            mlua::Value::UserData(data) => Self::Data(data),
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: std::any::type_name::<Self>().into(),
                    message: Some("expected callable value type".into()),
                });
            }
        })
    }
}
triv_conv_impl_both!(LuaCallableAny);

pub struct LuaCallable<A, R>(LuaCallableAny, PhantomData<fn(A) -> R>);
impl<A, R> std::fmt::Debug for LuaCallable<A, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl<A, R> Clone for LuaCallable<A, R> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }
}
impl<A: PushLuaMulti, R: PopLuaMulti> PopLua for LuaCallable<A, R> {
    type FromRepr = LuaCallableAny;
    fn from_mlua(repr: Self::FromRepr) -> Result<Self> {
        Ok(Self::cast_any_callable(repr))
    }
}
impl<A: PopLuaMulti, R: PushLuaMulti> PushLua for LuaCallable<A, R> {
    type IntoRepr = LuaCallableAny;
    fn into_mlua(self) -> Result<Self::IntoRepr> {
        Ok(self.into_any())
    }
}
impl<A, R> LuaCallable<A, R> {
    pub fn call_any_ret<U: PopLuaMulti>(self, args: impl LuaSubMulti<A>) -> Result<U>
    where
        A: FromLuaMultiTyped,
    {
        self.as_any().call_any(args)
    }

    pub fn call(&self, args: impl LuaSubMulti<A>) -> Result<R>
    where
        A: FromLuaMultiTyped,
        R: PopLuaMulti,
    {
        self.as_any().call_any(args)
    }

    pub fn cast_any_callable(func: LuaCallableAny) -> Self {
        Self(func, PhantomData)
    }
    pub fn from_mlua_func(func: mlua::Function) -> Self {
        Self::cast_any_callable(LuaCallableAny::Func(func))
    }
    pub fn into_any(self) -> LuaCallableAny {
        self.0
    }
    pub fn as_any(&self) -> &LuaCallableAny {
        &self.0
    }
}

pub struct LuaMap<K, V>(LuaTableAny, PhantomData<fn() -> (K, V)>);
pub struct LuaMapOwned<K, V>(LuaTableAny, PhantomData<fn() -> (K, V)>);
pub struct LuaMapMut<K, V>(
    LuaTableAny,
    #[allow(clippy::complexity)] PhantomData<fn(K, V) -> (K, V)>,
);
pub struct LuaSeq<T>(LuaTableAny, PhantomData<fn() -> T>);
pub struct LuaSeqMut<T>(LuaTableAny, PhantomData<fn(T) -> T>);
pub struct LuaSeqOwned<T>(LuaTableAny, PhantomData<fn() -> T>);
pub trait LuaTableGet {
    type Key;
    type Val;

    fn get(&self, key: impl LuaSub<Self::Key>) -> Result<Self::Val>;
}
pub trait LuaTableSet: LuaTableGet {
    fn set(&self, key: impl LuaSub<Self::Key>, val: impl LuaSub<Self::Val>) -> Result<()>;
}
impl<T: LuaTableGet> LuaTableGet for &T {
    type Key = T::Key;
    type Val = T::Val;

    fn get(&self, key: impl LuaSub<Self::Key>) -> Result<Self::Val> {
        T::get(self, key)
    }
}
impl<T: LuaTableSet> LuaTableSet for &T {
    fn set(&self, key: impl LuaSub<Self::Key>, val: impl LuaSub<Self::Val>) -> Result<()> {
        T::set(self, key, val)
    }
}

const _: () = {
    macro_rules! g_tbl_prox_impl_base {
        ((gp![$($g:tt)*], $t:ty, ($k:ty, $v:ty $(,)?) $(,)?)) => {
            impl<$($g)*> crate::lua::PushLua for $t
                where
                    $k: crate::lua::PushLua,
                    $v: crate::lua::PushLua,
            {
                type IntoRepr = mlua::Table;
                fn into_mlua(self) -> crate::lua::Result<Self::IntoRepr> {
                    self.into_table_any().into_mlua()
                }
            }
            #[allow(dead_code)]
            impl<$($g)*> $t {
                pub fn new(lua: &Lua) -> crate::lua::Result<Self> {
                    lua.create_table().map(Self::cast_table_any)
                }
                pub fn cast_table_any(table: LuaTableAny) -> Self {
                    Self(table, Default::default())
                }
                pub fn as_table_any(&self) -> &LuaTableAny {
                    &self.0
                }
                pub fn into_table_any(self) -> LuaTableAny {
                    self.0
                }
            }
            impl<$($g)*> LuaTableGet for $t
                where
                    $k: crate::lua::PopLua,
                    $v: crate::lua::PopLua,
            {
                type Key = $k;
                type Val = $v;

                fn get(&self, k: impl LuaSub<$k>) -> Result<$v> {
                    self.0.get_any(k)
                }
            }
            impl<$($g)*> Clone for $t {
                fn clone(&self) -> Self {
                    Self(self.0.clone(), self.1)
                }
            }
        };
    }
    macro_rules! g_tbl_prox_impl_const {
        ((gp![$($g:tt)*], $t:ty, ($k:ty, $v:ty $(,)?) $(,)?)) => {
            g_tbl_prox_impl_base![(gp![$($g)*], $t, ($k, $v))];

            impl<$($g)*> crate::lua::PopLua for $t
                where
                    $k: crate::lua::PopLua,
                    $v: crate::lua::PopLua,
            {
                type FromRepr = mlua::Table;
                fn from_mlua(repr: Self::FromRepr) -> crate::lua::Result<Self> {
                    Ok(Self::cast_table_any(LuaTableAny::from_mlua(repr)?))
                }
            }
        };
    }
    macro_rules! g_tbl_prox_impl_mut {
        ((gp![$($g:tt)*], $t:ty, ($k:ty, $v:ty $(,)?) $(,)?)) => {
            g_tbl_prox_impl_base![(gp![$($g)*], $t, ($k, $v))];

            impl<$($g)*> LuaTableSet for $t
            where
                $k: IntoLuaTyped + FromLuaTyped,
                $v: IntoLuaTyped + FromLuaTyped,
            {
                fn set(&self, k: impl LuaSub<$k>, v: impl LuaSub<$v>) -> Result<()> {
                    // TODO: Optional Validation
                    self.0.set_any(k, v)
                }
            }
            impl<$($g)*> crate::lua::PopLua for $t
                where
                    $k: crate::lua::PopLua + crate::lua::PushLua,
                    $v: crate::lua::PopLua + crate::lua::PushLua,
            {
                type FromRepr = mlua::Table;
                fn from_mlua(repr: Self::FromRepr) -> crate::lua::Result<Self> {
                    // TODO: Optional Validation
                    Ok(Self::cast_table_any(LuaTableAny::from_mlua(repr)?))
                }
            }
        };
    }
    macro_rules! g_tbl_prox_impl_owned {
        ((gp![$($g:tt)*], $t:ty, ($k:ty, $v:ty $(,)?) $(,)?)) => {
            g_tbl_prox_impl_base![(gp![$($g)*], $t, ($k, $v))];

            impl<$($g)*> LuaTableSet for $t
            where
                $k: FromLuaTyped,
                $v: FromLuaTyped,
            {
                fn set(&self, k: impl LuaSub<$k>, v: impl LuaSub<$v>) -> Result<()> {
                    // TODO: Optional Validation
                    self.0.set_any(k, v)
                }
            }
        };
    }
    g_tbl_prox_impl_const![(gp![K, V], LuaMap<K, V>, (K, V))];
    g_tbl_prox_impl_mut![(gp![K, V], LuaMapMut<K, V>, (K, V))];
    g_tbl_prox_impl_owned![(gp![K, V], LuaMapOwned<K, V>, (K, V))];
    g_tbl_prox_impl_const![(gp![T], LuaSeq<T>, (LuaInt, T))];
    g_tbl_prox_impl_mut![(gp![T], LuaSeqMut<T>, (LuaInt, T))];
    g_tbl_prox_impl_owned![(gp![T], LuaSeqOwned<T>, (LuaInt, T))];
};

pub struct LuaDefer<F>(pub F);
impl<T, F: FnOnce(&Lua) -> T> LuaDefer<F> {
    pub fn eval(self, lua: impl AsLua) -> T {
        self.0(lua.lua())
    }
}
pub fn lua_defer_val<T, F: FnOnce(&Lua) -> Result<T>>(f: F) -> LuaDefer<F> {
    LuaDefer(f)
}
macro_rules! LuaDeferImpl {
    ($res:ty) => {
        crate::lua::LuaDefer<impl FnOnce(&crate::lua::Lua) -> crate::lua::Result<$res>>
    };
}
pub(crate) use LuaDeferImpl;
impl<T: PushLua, F: FnOnce(&Lua) -> T> mlua::IntoLua for LuaDefer<F> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<LuaVal> {
        self.eval(lua)
            .into_mlua()
            .map_err(Into::into)
            .and_then(|it| mlua::IntoLua::into_lua(it, lua))
    }
}
impl<T: PushLua, F: FnOnce(&Lua) -> T> PushLua for LuaDefer<F> {
    type IntoRepr = Self;
    fn into_mlua(self) -> Result<Self::IntoRepr> {
        Ok(self)
    }
}

#[allow(dead_code)]
pub struct LuaCastIntoAny<T>(pub T);
impl<T: mlua::IntoLua> mlua::IntoLua for LuaCastIntoAny<T> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        self.0.into_lua(lua)
    }
}
impl<T: PushLua> PushLua for LuaCastIntoAny<T> {
    type IntoRepr = T::IntoRepr;
    fn into_mlua(self) -> Result<Self::IntoRepr> {
        self.0.into_mlua()
    }
}

pub fn lua_conv_sub<U: FromLuaTyped>(lua: &Lua, val: impl LuaSub<U>) -> Result<U> {
    lua.convert(val)
}

pub trait LuaStructInner: Sized {
    type Fields: FromLuaMultiTyped + IntoLuaMultiTyped;
    type NominalId: crate::typing::logic::BitSeq;
}
pub struct LuaStruct<T: LuaStructInner>(pub T);

impl<T: LuaStructInner + PushLua> PushLua for LuaStruct<T> {
    type IntoRepr = T::IntoRepr;
    fn into_mlua(self) -> Result<Self::IntoRepr> {
        self.0.into_mlua()
    }
}
impl<T: LuaStructInner + PopLua> PopLua for LuaStruct<T> {
    type FromRepr = T::FromRepr;
    fn from_mlua(repr: Self::FromRepr) -> Result<Self> {
        T::from_mlua(repr).map(Self)
    }
}

pub trait AsLua {
    fn lua(&self) -> &Lua;

    fn convert<R: PopLua>(&self, val: impl PushLua) -> Result<R> {
        self.lua()
            .as_mlua()
            .convert(val.into_mlua()?)
            .map_err(Into::into)
            .and_then(|it| R::from_mlua(it))
    }
}
impl<T: AsLua> AsLua for &T {
    fn lua(&self) -> &Lua {
        T::lua(self)
    }
}
impl AsLua for Lua {
    fn lua(&self) -> &Lua {
        self
    }
}
impl AsLua for mlua::Lua {
    fn lua(&self) -> &Lua {
        Lua::by_mlua(self)
    }
}
