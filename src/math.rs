use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Vec2D {
    pub x: f64,
    pub y: f64,
}

impl Vec2D {
    pub fn norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn norm2(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }
}

impl Add for Vec2D {
    type Output = Vec2D;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2D {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl Sub for Vec2D {
    type Output = Vec2D;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Vec2D {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Display for Vec2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

pub fn rect_ensure_positive_size(pos: Vec2D, size: Vec2D) -> (Vec2D, Vec2D) {
    let (pos_x, size_x) = if size.x > 0.0 {
        (pos.x, size.x)
    } else {
        ((pos.x + size.x), size.x.abs())
    };

    let (pos_y, size_y) = if size.y > 0.0 {
        (pos.y, size.y)
    } else {
        ((pos.y + size.y), size.y.abs())
    };

    (Vec2D::new(pos_x, pos_y), Vec2D::new(size_x, size_y))
}
