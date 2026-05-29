//! Implementation of the definitions in Section 2.1 of the WMF specifications.

mod enums;
mod flags;

pub use self::{enums::*, flags::*};

#[rustfmt::skip]
macro_rules! impl_parser {
    ($t:ident,u8) => {
        $crate::wmf::parser::constants::impl_parser!(_, $t, read_u8_from_le_bytes, 4);
    };
    ($t:ident,u16) => {
        $crate::wmf::parser::constants::impl_parser!(_, $t, read_u16_from_le_bytes, 6);
    };
    ($t:ident,u32) => {
        $crate::wmf::parser::constants::impl_parser!(_, $t, read_u32_from_le_bytes, 10);
    };
    ($t:ident,i32) => {
        $crate::wmf::parser::constants::impl_parser!(_, $t, read_i32_from_le_bytes, 10);
    };
    (_, $t:ident, $read_fn:ident, $digits:expr) => {
        impl $t {
            #[cfg_attr(feature = "tracing", ::tracing::instrument(
                level = tracing::Level::TRACE,
                skip_all,
                err(level = tracing::Level::ERROR, Display),
            ))]
            pub fn parse<R: $crate::wmf::Read>(
                buf: &mut R,
            ) -> Result<(Self, usize), $crate::wmf::parser::ParseError> {
                let (value, consumed_bytes) = crate::wmf::parser::$read_fn(buf)?;
                let Some(v) = Self::from_repr(value) else {
                    return Err($crate::wmf::parser::ParseError::UnexpectedEnumValue {
                        cause: format!(
                            ::core::concat!(
                                "unexpected value as ",
                                ::core::stringify!($t),
                                ": {:#0", $digits, "X}",
                            ),
                            value
                        ),
                    });
                };

                Ok((v, consumed_bytes))
            }
        }
    };
}

use impl_parser;
