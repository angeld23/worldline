use super::{
    transform::lorentz_factor,
    worldline::{Worldline, WorldlineEvent, PHYS_TIME_STEP},
};
use cgmath::{vec4, Matrix4, SquareMatrix, Vector4};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId(pub u128);

impl EntityId {
    pub fn generate() -> Self {
        Self(rand::random())
    }
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub worldline: Worldline,
    pub model: Option<String>,
    pub model_matrix: Matrix4<f32>,
    pub model_color: Vector4<f32>,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            worldline: Worldline::default(),
            model: None,
            model_matrix: Matrix4::identity(),
            model_color: vec4(1.0, 1.0, 1.0, 1.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Universe {
    pub entities: BTreeMap<EntityId, Entity>,
    pub user_entity_id: EntityId,
    pub time: f64,
}

impl Default for Universe {
    fn default() -> Self {
        let user_entity_id = EntityId::generate();

        let mut entities = BTreeMap::new();
        entities.insert(user_entity_id, Entity::default());

        Self {
            entities,
            user_entity_id,
            time: 1000.0,
        }
    }
}

impl Universe {
    pub fn get_user_entity(&self) -> &Entity {
        self.entities.get(&self.user_entity_id).unwrap()
    }

    pub fn get_user_entity_mut(&mut self) -> &mut Entity {
        self.entities.get_mut(&self.user_entity_id).unwrap()
    }

    pub fn insert_entity(&mut self, entity: Entity) -> EntityId {
        let entity_id = EntityId::generate();
        self.entities.insert(entity_id, entity);
        entity_id
    }

    pub fn remove_entity(&mut self, entity_id: EntityId) -> Option<Entity> {
        if entity_id == self.user_entity_id {
            return None;
        }

        self.entities.remove(&entity_id)
    }

    pub fn user_event_now(&self) -> WorldlineEvent {
        self.get_user_entity()
            .worldline
            .get_event_at_time(self.time)
    }

    pub fn step(&mut self, delta: f64) {
        let user_event = self.user_event_now();
        let user_frame = user_event.frame;
        let user_gamma = lorentz_factor(user_frame.velocity);

        self.time += delta * user_gamma;

        self.entities.par_iter_mut().for_each(|(_, entity)| {
            entity.worldline.time_resolution = PHYS_TIME_STEP * user_gamma;
            entity.worldline.bake_events(self.time);
        });
    }
}
