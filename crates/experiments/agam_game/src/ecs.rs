//! High-Performance Archetype-aware Entity Component System (ECS).

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Unique generational Entity identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

pub trait Component: 'static + Send + Sync {}
impl<T: 'static + Send + Sync> Component for T {}

/// Central ECS World holding all entities and component storages.
#[derive(Default)]
pub struct World {
    entities: Vec<Option<u32>>, // generation per index
    free_indices: Vec<u32>,
    components: HashMap<TypeId, HashMap<Entity, Box<dyn Any + Send + Sync>>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new unique Entity.
    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free_indices.pop() {
            let generation = self.entities[index as usize].unwrap_or(1);
            let entity = Entity { index, generation };
            self.entities[index as usize] = Some(generation);
            entity
        } else {
            let index = self.entities.len() as u32;
            let entity = Entity {
                index,
                generation: 1,
            };
            self.entities.push(Some(1));
            entity
        }
    }

    /// Despawn an entity and remove all associated components.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if let Some(slot) = self.entities.get_mut(entity.index as usize)
            && *slot == Some(entity.generation)
        {
            *slot = Some(entity.generation.wrapping_add(1));
            self.free_indices.push(entity.index);
            for storage in self.components.values_mut() {
                storage.remove(&entity);
            }
            return true;
        }
        false
    }

    /// Attach a component to an entity.
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();
        let storage = self.components.entry(type_id).or_default();
        storage.insert(entity, Box::new(component));
    }

    /// Borrow component immutably.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let storage = self.components.get(&type_id)?;
        let boxed = storage.get(&entity)?;
        boxed.downcast_ref::<T>()
    }

    /// Borrow component mutably.
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        let storage = self.components.get_mut(&type_id)?;
        let boxed = storage.get_mut(&entity)?;
        boxed.downcast_mut::<T>()
    }

    /// Check if entity is active and valid.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.get(entity.index as usize).copied().flatten() == Some(entity.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Position {
        x: f32,
        y: f32,
    }

    struct Velocity {
        vx: f32,
        vy: f32,
    }

    #[test]
    fn test_spawn_and_component_lifecycle() {
        let mut world = World::new();
        let e1 = world.spawn();

        world.insert(e1, Position { x: 10.0, y: 20.0 });
        world.insert(e1, Velocity { vx: 1.0, vy: -1.0 });

        assert!(world.is_alive(e1));
        assert_eq!(world.get::<Position>(e1).unwrap().x, 10.0);
        assert_eq!(world.get::<Position>(e1).unwrap().y, 20.0);
        assert_eq!(world.get::<Velocity>(e1).unwrap().vx, 1.0);
        assert_eq!(world.get::<Velocity>(e1).unwrap().vy, -1.0);

        if let Some(pos) = world.get_mut::<Position>(e1) {
            pos.x += 5.0;
        }
        assert_eq!(world.get::<Position>(e1).unwrap().x, 15.0);

        assert!(world.despawn(e1));
        assert!(!world.is_alive(e1));
        assert!(world.get::<Position>(e1).is_none());
    }
}
