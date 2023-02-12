use super::*;

impl Editor {
    /// Undoes the last action and puts its reverse in the redo stack.
    pub fn undo(&mut self) {
        if let Some(mut state) = self.undo_stack.pop() {
            std::mem::swap(&mut self.world.level, &mut state);
            self.redo_stack.push(state);
            self.update_geometry();
        }
    }

    /// Redoes the last undid action and puts its reverse in the undo stack.
    pub fn redo(&mut self) {
        if let Some(mut state) = self.redo_stack.pop() {
            std::mem::swap(&mut self.world.level, &mut state);
            self.undo_stack.push(state);
            self.update_geometry();
        }
    }

    /// Remembers the current state in the undo stack and clear the redo stack.
    pub fn keep_state(&mut self) {
        self.redo_stack.clear();
        self.undo_stack.push(self.world.level.clone());
    }

    pub fn action_place(&mut self, block: PlaceableType, position: vec2<Coord>) {
        let grid_pos = self.world.level.grid.world_to_grid(position).0;
        match block {
            PlaceableType::Tile(tile) => {
                self.world
                    .level
                    .tiles
                    .set_tile_isize(grid_pos, tile, &self.assets);
            }
            PlaceableType::Hazard(hazard) => {
                self.world.level.place_hazard(position, hazard);
            }
            PlaceableType::Coin => {
                self.world.level.place_coin(grid_pos);
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
                self.world.level.place_prop(grid_pos, size, prop);
            }
            PlaceableType::Spotlight(light) => {
                self.world
                    .level
                    .spotlights
                    .push(SpotlightSource { position, ..light });
            }
        };
        self.update_geometry();
    }

    pub fn action_remove<'a>(&mut self, ids: impl IntoIterator<Item = &'a PlaceableId>) {
        self.world.level.remove_blocks(ids, &self.assets);
        self.hovered.clear();
        self.update_geometry();
    }
}
