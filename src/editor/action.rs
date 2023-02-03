use super::*;

#[derive(Debug, Clone)]
pub enum Action {
    Place {
        block: PlaceableType,
        pos: vec2<Coord>,
    },
    Remove {
        pos: vec2<Coord>,
    },
    Replace(Placeable),
}

impl Editor {
    pub fn action(&mut self, action: Action) {
        self.redo_actions.clear();
        let undo_action = self.action_impl(action);
        self.undo_actions.extend(undo_action);
    }

    fn action_impl(&mut self, action: Action) -> Vec<Action> {
        let actions = match action {
            Action::Place { block, pos } => self.action_place(block, pos),

            Action::Remove { pos } => self.action_remove(pos),
            Action::Replace(block) => self.action_replace(block),
        };
        self.update_geometry();
        actions
    }

    pub fn undo(&mut self) {
        if let Some(action) = self.undo_actions.pop() {
            let redo_action = self.action_impl(action);
            self.redo_actions.extend(redo_action);
        }
    }

    pub fn redo(&mut self) {
        if let Some(action) = self.redo_actions.pop() {
            let undo_action = self.action_impl(action);
            self.undo_actions.extend(undo_action);
        }
    }

    fn action_place(&mut self, block: PlaceableType, position: vec2<Coord>) -> Vec<Action> {
        let grid_pos = self.level.grid.world_to_grid(position).0;
        match block {
            PlaceableType::Tile(tile) => {
                self.level
                    .tiles
                    .set_tile_isize(grid_pos, tile, &self.assets);
            }
            PlaceableType::Hazard(hazard) => {
                self.level.place_hazard(grid_pos, hazard);
            }
            PlaceableType::Coin => {
                self.level.place_coin(grid_pos);
            }
            PlaceableType::Prop(prop) => {
                let size = self
                    .assets
                    .sprites
                    .props
                    .get_texture(&prop)
                    .size()
                    .map(|x| x as f32 / PIXELS_PER_UNIT as f32)
                    .map(Coord::new);
                self.level.place_prop(grid_pos, size, prop);
            }
            PlaceableType::Spotlight(light) => self
                .level
                .spotlights
                .push(SpotlightSource { position, ..light }),
        }
        vec![]
    }

    fn action_replace(&mut self, block: Placeable) -> Vec<Action> {
        match block {
            Placeable::Tile((tile, pos)) => {
                self.level.tiles.set_tile_isize(pos, tile, &self.assets);
            }
            Placeable::Hazard(hazard) => {
                self.level.hazards.push(hazard);
            }
            Placeable::Coin(coin) => {
                self.level.coins.push(coin);
            }
            Placeable::Prop(prop) => {
                self.level.props.push(prop);
            }
            Placeable::Spotlight(spotlight) => self.level.spotlights.push(spotlight),
        }
        vec![]
    }

    fn action_remove(&mut self, _pos: vec2<Coord>) -> Vec<Action> {
        let actions = self
            .level
            .remove_blocks(&self.hovered, &self.assets)
            .into_iter()
            .map(Action::Replace)
            .collect();
        self.hovered.clear();
        actions
    }
}
