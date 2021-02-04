use super::SharedNvim;
use anyhow::Result;
use nvim_rs::{call_args, rpc::model::IntoVal, Value};

macro_rules! wrap_api {
    (
        $vis:vis async fn $fn_name:ident(
            $nvim:ident: $nvim_ty:ty,
            $($arg:tt: $type:ty),*)
            -> $result_ty:ident<$ok_ty:tt $(, $err_ty:tt)?>;
    ) => {
        $vis async fn $fn_name($nvim: $nvim_ty, $($arg: $type),*) -> $result_ty<$ok_ty $(, $err_ty)?> {
            use nvim_rs::rpc::unpack::TryUnpack;
            use nvim_rs::Value;

            $(let $arg = Value::from($arg);)*
            let res = $nvim.call(stringify!($fn_name), vec![$($arg),*]).await?.expect("Failed to get value");
            Ok(res.try_unpack().expect("Failed to unpack"))
        }
    };

    (
        $(
            $vis:vis async fn $fn_name:ident($($stuff:tt)*) -> $result_ty:ident<$ok_ty:ty $(, $err_ty:ty)?>;
        )*
    ) => {
        $(wrap_api! { $vis async fn $fn_name($($stuff)*) -> $result_ty<$ok_ty $(, $err_ty)?>; })*
    };

    (
        $vis:vis async fn $fn_name:ident(
            $nvim:ident: $nvim_ty:ty,
            $($arg:tt: $type:ty),*)
            -> $result_ty:ident<() $(, $err_ty:tt)?>;
    ) => {
        $vis async fn $fn_name($nvim: $nvim_ty, $($arg: $type),*) -> $result_ty<$ok_ty $(, $err_ty)?> {
            use nvim_rs::rpc::unpack::TryUnpack;
            use nvim_rs::Value;

            $(let $arg = Value::from($arg);)*
            $nvim.call(stringify!($fn_name), vec![$($arg),*]).await?;
        }
    };
}

macro_rules! wrap_fn {
    (
        $vis:vis async fn $fn_name:ident(
            $nvim:ident: $nvim_ty:ty,
            $($arg:tt: $type:ty),*)
            -> $result_ty:ident<$ok_ty:tt $(, $err_ty:tt)?>;
    ) => {
        $vis async fn $fn_name($nvim: $nvim_ty, $($arg: $type),*) -> $result_ty<$ok_ty $(, $err_ty)?> {
            use nvim_rs::rpc::unpack::TryUnpack;
            use nvim_rs::Value;

            $(let $arg = Value::from($arg);)*
            let res = $nvim.call_function(stringify!($fn_name), vec![$($arg),*]).await?;
            Ok(res.try_unpack().expect("Failed to unpack"))
        }
    };

    (
        $vis:vis async fn $fn_name:ident(
            $nvim:ident: $nvim_ty:ty,
            $($arg:tt: $type:ty),*)
            -> $result_ty:ident<() $(, $err_ty:tt)?>;
    ) => {
        $vis async fn $fn_name($nvim: $nvim_ty, $($arg: $type),*) -> $result_ty<$ok_ty $(, $err_ty)?> {
            use nvim_rs::rpc::unpack::TryUnpack;
            use nvim_rs::Value;

            $(let $arg = Value::from($arg);)*
            $nvim.call_function(stringify!($fn_name), vec![$($arg),*]).await?;
        }
    };

    (
        $(
            $vis:vis async fn $fn_name:ident($($stuff:tt)*) -> $result_ty:ident<$ok_ty:ty $(, $err_ty:ty)?>;
        )*
    ) => {
        $(wrap_fn! { $vis async fn $fn_name($($stuff)*) -> $result_ty<$ok_ty $(, $err_ty)?>; })*
    }
}

wrap_fn! {
    pub async fn line(nvim: SharedNvim, mark: &str) -> Result<i64>;

    pub async fn col(nvim: SharedNvim, mark: &str) -> Result<i64>;

    pub async fn line2byte(nvim: SharedNvim, line: i64) -> Result<i64>;
}

wrap_api! {
    pub async fn nvim_complete(nvim: SharedNvim, col: i64, entries: Vec<Value>, opts: Vec<(Value, Value)>) -> Result<()>;
}

pub async fn get_byte_offset(nvim: SharedNvim) -> Result<i64> {
    let line = line(nvim.clone(), ".").await?;
    let col = col(nvim.clone(), ".").await?;
    let res = line2byte(nvim.clone(), line).await? + col - 2;
    Ok(res)
}
