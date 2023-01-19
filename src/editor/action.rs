use super::*;

#[derive(Debug, Clone)]
pub enum Action {
    Place { block: BlockType, pos: Vec2<Coord> },
    Remove { pos: Vec2<Coord> },
    Replace(Block),
}

impl Editor {
    pub fn action(&mut self, action: Action) {
        self.redo_actions.clear();
        let undo_action = self.action_impl(action);
        self.undo_actions.extend(undo_action);
    }

    fn action_impl(&mut self, action: Action) -> Vec<Action> {
        match action {
            Action::Place { block, pos } => self.action_place(block, pos),
            Action::Remove { pos } => self.action_remove(pos),
            Action::Replace(block) => self.action_replace(block),
        }
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

    fn action_place(&mut self, block: BlockType, pos: Vec2<Coord>) -> Vec<Action> {
        let pos = self.level.grid.world_to_grid(pos).0;
        match block {
            BlockType::Tile(tile) => {
                self.level.tiles.set_tile_isize(pos, tile);
            }
            BlockType::Hazard(hazard) => {
                self.level.place_hazard(pos, hazard);
            }
            BlockType::Coin => {
                self.level.place_coin(pos);
            }
            BlockType::Prop(prop) => {
                let size = self
                    .assets
                    .sprites
                    .props
                    .get_texture(&prop)
                    .size()
                    .map(|x| x as f32 / PIXELS_PER_UNIT)
                    .map(Coord::new);
                self.level.place_prop(pos, size, prop);
            }
        }
        vec![]
    }

    fn action_replace(&mut self, block: Block) -> Vec<Action> {
        match block {
            Block::Tile((tile, pos)) => {
                self.level.tiles.set_tile_isize(pos, tile);
            }
            Block::Hazard(hazard) => {
                self.level.hazards.push(hazard);
            }
            Block::Coin(coin) => {
                self.level.coins.push(coin);
            }
            Block::Prop(prop) => {
                self.level.props.push(prop);
            }
        }
        vec![]
    }

    fn action_remove(&mut self, pos: Vec2<Coord>) -> Vec<Action> {
        self.level
            .remove_all_at(pos)
            .into_iter()
            .map(Action::Replace)
            .collect()
    }
}
