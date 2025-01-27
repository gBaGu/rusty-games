use bevy::prelude::*;

/// Value that triggers [`ValueUpdated`] event whenever it's changed.
#[derive(Debug, Component, Deref)]
pub struct WatchedValue<T>(Option<T>);

impl<T> Default for WatchedValue<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T> From<Option<T>> for WatchedValue<T> {
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

impl<T: PartialEq> PartialEq<Mirror<T>> for WatchedValue<T> {
    fn eq(&self, other: &Mirror<T>) -> bool {
        **self == **other
    }
}

impl<T> WatchedValue<T> {
    pub fn set(&mut self, value: T) {
        self.0 = Some(value);
    }

    pub fn reset(&mut self) {
        self.0 = None;
    }
}

/// Copy of a [`WatchedValue`]. Needed to detect change.
#[derive(Debug, Component, Deref, DerefMut)]
struct Mirror<T>(Option<T>);

impl<T> Default for Mirror<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T> From<Option<T>> for Mirror<T> {
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

impl<T: Copy> Mirror<T> {
    pub fn sync(&mut self, storage: &WatchedValue<T>) {
        self.0 = **storage;
    }
}

#[derive(Debug, Bundle)]
pub struct WatchedValueBundle<T: Send + Sync + 'static> {
    storage: WatchedValue<T>,
    mirror: Mirror<T>,
}

impl<T: Send + Sync + 'static> Default for WatchedValueBundle<T> {
    fn default() -> Self {
        Self {
            storage: Default::default(),
            mirror: Default::default(),
        }
    }
}

/// Signals that the value inside [`WatchedValue`] was updated.
/// Contains an entity that triggered this event and new value.
#[derive(Debug, Event)]
pub struct ValueUpdated<T> {
    source: Entity,
    value: Option<T>,
}

impl<T> ValueUpdated<T> {
    fn new(source: Entity, value: Option<T>) -> Self {
        Self { source, value }
    }

    pub fn source(&self) -> Entity {
        self.source
    }

    pub fn value(&self) -> &Option<T> {
        &self.value
    }
}

/// System that checks for a value change and sends [`ValueUpdated`] if it changed.
fn check_updates<T: Copy + PartialEq + Send + Sync + 'static>(
    mut storage: Query<(Entity, &mut Mirror<T>, &WatchedValue<T>), Changed<WatchedValue<T>>>,
    mut storage_updated: EventWriter<ValueUpdated<T>>,
) {
    for (storage_entity, mut mirror, storage) in storage.iter_mut() {
        let mirror = mirror.as_mut();
        if storage != mirror {
            mirror.sync(storage);
            storage_updated.send(ValueUpdated::new(storage_entity, **storage));
        }
    }
}

/// Helper function to set up event and system required to process values of type `T`.
pub fn setup<T: Copy + PartialEq + Send + Sync + 'static>(app: &mut App) -> &mut App {
    app.add_event::<ValueUpdated<T>>()
        .add_systems(Update, check_updates::<T>)
}
