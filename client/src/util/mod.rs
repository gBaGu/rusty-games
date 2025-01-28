pub mod watched_value;

macro_rules! entity_type {
    (
        $(#[$($attr:tt)*])*
        $i:ident, $derive_trait:ty
    ) => {
        $(#[$($attr)*])*
        #[derive(Clone, Copy, Debug, $derive_trait)]
        pub struct $i(Entity);

        impl From<Entity> for $i {
            fn from(value: Entity) -> Self {
                Self(value)
            }
        }

        impl $i {
            pub fn get(&self) -> Entity {
                self.0
            }
        }
    };
}

pub(crate) use entity_type;
