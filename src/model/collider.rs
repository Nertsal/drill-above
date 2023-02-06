use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Collider(Aabb2<Coord>);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Collision {
    pub normal: vec2<Coord>,
    pub penetration: Coord,
    pub offset: Coord,
}

impl Collider {
    pub fn new(aabb: Aabb2<Coord>) -> Self {
        Self(aabb)
    }

    pub fn raw(&self) -> Aabb2<Coord> {
        self.0
    }

    pub fn pos(&self) -> vec2<Coord> {
        self.0.center()
    }

    pub fn size(&self) -> vec2<Coord> {
        self.0.size()
    }

    pub fn feet(&self) -> vec2<Coord> {
        self.pos() - vec2(Coord::ZERO, self.size().y / Coord::new(2.0))
    }

    pub fn head(&self) -> vec2<Coord> {
        self.pos() + vec2(Coord::ZERO, self.size().y / Coord::new(2.0))
    }

    pub fn grid_aabb(&self, grid: &Grid) -> Aabb2<isize> {
        let eps = Coord::new(1e-3);
        let [a, b] = [self.0.bottom_left(), self.0.top_right() - vec2(eps, eps)]
            .map(|p| grid.world_to_grid(p).0);
        Aabb2::from_corners(a, b)
    }

    pub fn teleport(&mut self, position: vec2<Coord>) {
        let delta = position - self.feet();
        self.translate(delta);
    }

    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.0 = self.0.translate(delta);
    }

    pub fn check(&self, other: &Self) -> bool {
        self.0.intersects(&other.0)
    }

    pub fn collide(&self, other: &Self) -> Option<Collision> {
        if !self.check(other) {
            return None;
        }

        let dx_right = self.0.max.x - other.0.min.x;
        let dx_left = other.0.max.x - self.0.min.x;
        let dy_up = self.0.max.y - other.0.min.y;
        let dy_down = other.0.max.y - self.0.min.y;

        let (nx, px) = if dx_right < dx_left {
            (-Coord::ONE, dx_right)
        } else {
            (Coord::ONE, dx_left)
        };
        let (ny, py) = if dy_up < dy_down {
            (-Coord::ONE, dy_up)
        } else {
            (Coord::ONE, dy_down)
        };

        if px <= Coord::ZERO || py <= Coord::ZERO {
            None
        } else if px < py {
            Some(Collision {
                normal: vec2(nx, Coord::ZERO),
                penetration: px,
                offset: py * ny,
            })
        } else {
            Some(Collision {
                normal: vec2(Coord::ZERO, ny),
                penetration: py,
                offset: px * nx,
            })
        }
    }
}
