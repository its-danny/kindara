use bevy::prelude::*;
use fake::{Dummy, Fake, Faker};

use crate::{
    combat::bundles::CombatBundle,
    interact::components::{Interaction, Interactions},
    npc::{
        bundles::{HostileBundle, NpcBundle},
        components::{Friendly, Hostile, Npc},
    },
    visual::components::Depiction,
};

#[derive(Dummy)]
pub struct NpcBuilder {
    name: String,
    short_name: String,
    description: String,
    tags: Vec<String>,
    #[dummy(expr = "None")]
    interactions: Option<Vec<Interaction>>,
    #[dummy(expr = "None")]
    tile: Option<Entity>,
    #[dummy(expr = "false")]
    combat: bool,
    skills: Vec<String>,
}

#[allow(dead_code)]
impl NpcBuilder {
    pub fn new() -> Self {
        Faker.fake::<Self>()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn short_name(mut self, short_name: &str) -> Self {
        self.short_name = short_name.to_string();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.iter().map(|t| t.to_string()).collect();
        self
    }

    pub fn interactions(mut self, interactions: Vec<Interaction>) -> Self {
        self.interactions = Some(interactions);
        self
    }

    pub fn tile(mut self, tile: Entity) -> Self {
        self.tile = Some(tile);
        self
    }

    /// Gives the entity the Hostile bundle and an Attack interaction.
    pub fn combat(mut self, combat: bool) -> Self {
        self.combat = combat;
        self
    }

    pub fn skills(mut self, skills: Vec<&str>) -> Self {
        self.skills = skills.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn build(self, app: &mut App) -> Entity {
        let mut entity = app.world.spawn(NpcBundle {
            npc: Npc,
            interactions: Interactions(vec![]),
            depiction: Depiction {
                name: self.name,
                short_name: self.short_name,
                description: self.description,
                tags: self.tags,
                visible: true,
            },
        });

        if let Some(tile) = self.tile {
            entity.set_parent(tile);
        }

        if let Some(interactions) = self.interactions {
            entity.insert(Interactions(interactions));
        }

        if self.combat {
            entity.insert((HostileBundle {
                hostile: Hostile {
                    skills: self.skills,
                },
                combat: CombatBundle::default(),
            },));

            let interactions = entity.get_mut::<Interactions>();

            if let Some(mut interactions) = interactions {
                interactions.0.push(Interaction::Attack);
            } else {
                entity.insert(Interactions(vec![Interaction::Attack]));
            }
        } else {
            entity.insert(Friendly);
        }

        entity.id()
    }
}
