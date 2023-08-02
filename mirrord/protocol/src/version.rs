pub mod try_parse;

#[macro_export]
macro_rules! impl_versioned {
    ($inner:ident, $name:ident, $extend:ty) => {
        impl $name {
            pub fn unpack(self) -> ($inner, Option<$extend>) {
                (self.inner, self.extensions.into())
            }
        }

        impl From<$inner> for $name {
            fn from(inner: $inner) -> Self {
                Self {
                    inner,
                    extensions: $crate::version::try_parse::TryParse::NotPresent,
                }
            }
        }

        impl From<($inner, $extend)> for $name {
            fn from((inner, extensions): ($inner, $extend)) -> Self {
                Self {
                    inner,
                    extensions: $crate::version::try_parse::TryParse::Parsed(extensions),
                }
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &<Self as std::ops::Deref>::Target {
                &self.inner
            }
        }
    };
}

#[macro_export]
macro_rules! define_versioned {
    ($(#[$pre:ident $($attr:tt)*])* $vis:vis struct $name:ident $extend:ty { $($definition:tt)* }) => {
        ::paste::paste! {
            $(#[$pre $($attr)*])*
            $vis struct [<Inner $name:camel>] {
                $($definition)*
            }

            $(#[$pre $($attr)*])*
            $vis struct $name {
                pub inner: [<Inner $name:camel>],
                pub extensions: $crate::version::try_parse::TryParse<$extend>,
            }

            $crate::impl_versioned!([<Inner $name:camel>], $name, $extend);
        }
    };
    ($(#[$pre:ident $($attr:tt)*])* $vis:vis enum $name:ident $extend:ty { $($definition:tt)* }) => {
        ::paste::paste! {
            $(#[$pre $($attr)*])*
            $vis enum [<$name:camel Kind>] {
                $($definition)*
            }

            $(#[$pre $($attr)*])*
            $vis struct $name {
                pub inner: [<$name:camel Kind>],
                pub extensions: TryParse<$extend>,
            }

            $crate::impl_versioned!([<Inner $name:camel>], $name, $extend);
        }
    };
    ($(#[$pre:ident $($attr:tt)*])* $type:ident $extend:ty { $version:ident -> $definition:tt $($version_rest:ident -> $definition_rest:tt)+ }) => {
        $crate::define_versioned!($(#[$pre $($attr)*])* pub $type $version $extend $definition);
        $crate::define_versioned!($(#[$pre $($attr)*])* $type $version { $($version_rest -> $definition_rest)* });
    };
    ($(#[$pre:ident $($attr:tt)*])* $type:ident $extend:ty { $version:ident -> $definition:tt }) => {
        $crate::define_versioned!($(#[$pre $($attr)*])* pub $type $version $extend $definition);
        pub type Owned = $version;
    };
    ($(#[$pre:ident $($attr:tt)*])* $vis:vis $type:ident $name:ident { $($version:tt -> $definition:tt);+ }) => {
        ::paste::paste! {
            $vis mod [<$name:snake>] {
                use super::*;

                $crate::define_versioned!($(#[$pre $($attr)*])* $type () { $( $version -> $definition)+ });
            }
            $vis type $name = [<$name:snake>] :: Owned;
        }
    };
}

#[macro_export]
macro_rules! fast_versioned {
    ($(#[$pre:ident $($attr:tt)*])* $vis:vis $type:ident $name:ident { $($definition:tt)* }) => {
        $crate::define_versioned!($(#[$pre $($attr)*])* $vis $type $name {V1 -> { $($definition)* } });
    };
}
