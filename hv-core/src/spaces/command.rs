//! A buffer for deferred spawns and other actions on a [`Space`].
//!
//! Many [`Space`] operations such as spawning, despawning, and component insertion/removal require
//! mutable access to the [`Space`] in question and cannot be called while querying on the space,
//! whether by [`Space::query_mut`] or [`Space::query`]. [`CommandBuffer`] provides a convenient
//! abstraction for deferring these operations so that they can be called with a very similar API
//! while a query is happening. There are some caveats such as that newly spawned object IDs cannot
//! be immediately known; the builtin queueing functionality of [`Space`] does cover this, so you
//! likely won't need a [`CommandBuffer`] in most cases, as [`Space`] contains a convenient one.

use hecs::{Bundle, DynamicBundle, EntityBuilder};

use crate::{
    error::*,
    spaces::{ComponentError, Object, Space, SpaceId},
};

struct SpawnCommand {
    builder: EntityBuilder,
}

struct DespawnCommand {
    target: Object,
}

struct InsertCommand {
    target: Object,
    builder: EntityBuilder,
}

struct RemoveCommand {
    target: Object,
    remove: fn(Object, SpaceId, &mut hecs::World) -> Result<(), ComponentError>,
}

enum Command {
    Spawn(SpawnCommand),
    Despawn(DespawnCommand),
    Insert(InsertCommand),
    Remove(RemoveCommand),
}

/// A buffer containing queued commands to be later executed on a [`Space`]. Useful for when you
/// want to queue object spawning/component insertion/removal/despawning while querying a space.
#[derive(Default)]
pub struct CommandBuffer {
    entity_builder_pool: Vec<EntityBuilder>,
    queue: Vec<Command>,
}

impl CommandBuffer {
    /// Create an empty command buffer.
    pub fn new() -> Self {
        Self {
            entity_builder_pool: Vec::new(),
            queue: Vec::new(),
        }
    }

    /// Get an entity builder from the internal pool of the command buffer.
    pub(crate) fn get_builder(&mut self) -> EntityBuilder {
        self.entity_builder_pool
            .pop()
            .unwrap_or_else(EntityBuilder::new)
    }

    /// Queue an object spawn. You don't immediately get the object ID from this, so if you need the
    /// object ID immediately, you might want to use [`Space::reserve_object`] and a
    /// [`CommandBuffer::insert`] instead.
    pub fn spawn(&mut self, bundle: impl DynamicBundle) {
        let mut builder = self.get_builder();
        builder.add_bundle(bundle);
        self.spawn_builder(builder);
    }

    /// Queue an object spawn with an `EntityBuilder`, passing ownership of the builder to the
    /// command buffer's internal pool of `EntityBuilder`s.
    pub(crate) fn spawn_builder(&mut self, builder: EntityBuilder) {
        self.queue.push(Command::Spawn(SpawnCommand { builder }));
    }

    /// Queue an object despawn.
    pub fn despawn(&mut self, target: Object) {
        self.queue.push(Command::Despawn(DespawnCommand { target }));
    }

    /// Queue insertion of components onto an object.
    pub fn insert(&mut self, target: Object, bundle: impl DynamicBundle) {
        let mut builder = self.get_builder();
        builder.add_bundle(bundle);
        self.insert_builder(target, builder);
    }

    /// Queue a component insertion with an `EntityBuilder`, passing ownership of the builder to the
    /// command buffer's internal pool of `EntityBuilder`s.
    pub(crate) fn insert_builder(&mut self, target: Object, builder: EntityBuilder) {
        self.queue
            .push(Command::Insert(InsertCommand { target, builder }));
    }

    /// Queue removal of components onto an object.
    pub fn remove<T: Bundle + 'static>(&mut self, target: Object) {
        fn remover<T: Bundle + 'static>(
            object: Object,
            space_id: SpaceId,
            ecs: &mut hecs::World,
        ) -> Result<(), ComponentError> {
            if object.space != space_id {
                Err(ComponentError::WrongSpace)
            } else {
                ecs.remove::<T>(object.entity)?;
                Ok(())
            }
        }

        self.queue.push(Command::Remove(RemoveCommand {
            target,
            remove: remover::<T>,
        }));
    }

    pub(super) fn run_internal(&mut self, space_id: SpaceId, ecs: &mut hecs::World) -> Result<()> {
        let mut errors = Vec::new();
        for command in self.queue.drain(..) {
            let maybe_err = match command {
                Command::Spawn(mut spawn_cmd) => {
                    ecs.spawn(spawn_cmd.builder.build());
                    self.entity_builder_pool.push(spawn_cmd.builder);
                    None
                }
                Command::Despawn(despawn_cmd) => {
                    if despawn_cmd.target.space != space_id {
                        Some(ComponentError::WrongSpace)
                    } else {
                        ecs.despawn(despawn_cmd.target.entity).err().map(From::from)
                    }
                }
                Command::Insert(mut insert_cmd) => {
                    if insert_cmd.target.space != space_id {
                        Some(ComponentError::WrongSpace)
                    } else {
                        let maybe_err = ecs
                            .insert(insert_cmd.target.entity, insert_cmd.builder.build())
                            .err();
                        self.entity_builder_pool.push(insert_cmd.builder);
                        maybe_err.map(From::from)
                    }
                }
                Command::Remove(remove_cmd) => {
                    (remove_cmd.remove)(remove_cmd.target, space_id, ecs).err()
                }
            };

            errors.extend(maybe_err);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(
                "errors occurred while running queued commands: {:?}",
                errors
            ))
        }
    }

    /// Drain this command buffer and run all commands in it on a [`Space`]. All commands will be
    /// run, even if a command fails; errors will be reported together afterwards.
    pub fn run(&mut self, space: &mut Space) -> Result<()> {
        self.run_internal(space.id, &mut space.ecs)
    }
}
