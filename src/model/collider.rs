use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Collider(AABB<Coord>);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Collision {
    pub normal: Vec2<Coord>,
    pub penetration: Coord,
}

impl Collider {
    pub fn new(aabb: AABB<Coord>) -> Self {
        Self(aabb)
    }

    pub fn raw(&self) -> AABB<Coord> {
        self.0
    }

    pub fn pos(&self) -> Vec2<Coord> {
        self.0.center()
    }

    pub fn size(&self) -> Vec2<Coord> {
        self.0.size()
    }

    pub fn feet(&self) -> Vec2<Coord> {
        self.pos() - vec2(Coord::ZERO, self.size().y / Coord::new(2.0))
    }

    pub fn grid_aabb(&self, grid: &Grid) -> AABB<isize> {
        let [a, b] = [self.0.bottom_left(), self.0.top_right()].map(|p| grid.world_to_grid(p).0);
        AABB::from_corners(a, b)
    }

    pub fn teleport(&mut self, position: Vec2<Coord>) {
        let delta = position - self.feet();
        self.translate(delta);
    }

    pub fn translate(&mut self, delta: Vec2<Coord>) {
        self.0 = self.0.translate(delta);
    }

    pub fn check(&self, other: &Self) -> Option<Collision> {
        let dx_right = self.0.x_max - other.0.x_min;
        let dx_left = other.0.x_max - self.0.x_min;
        let dy_up = self.0.y_max - other.0.y_min;
        let dy_down = other.0.y_max - self.0.y_min;

        let (nx, px) = if dx_right < dx_left {
            (Coord::ONE, dx_right)
        } else {
            (-Coord::ONE, dx_left)
        };
        let (ny, py) = if dy_up < dy_down {
            (Coord::ONE, dy_up)
        } else {
            (-Coord::ONE, dy_down)
        };

        if px <= Coord::ZERO || py <= Coord::ZERO {
            None
        } else if px < py {
            Some(Collision {
                normal: vec2(nx, Coord::ZERO),
                penetration: px,
            })
        } else {
            Some(Collision {
                normal: vec2(Coord::ZERO, ny),
                penetration: py,
            })
        }
    }
}
