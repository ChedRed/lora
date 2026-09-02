use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

// region Add

impl Add for Vector2 {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Add<f32> for Vector2 {
    type Output = Self;
    fn add(self, other: f32) -> Self::Output {
        Self {
            x: self.x + other,
            y: self.y + other,
        }
    }
}

impl AddAssign for Vector2 {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}

impl AddAssign<f32> for Vector2 {
    fn add_assign(&mut self, other: f32) {
        *self = Self {
            x: self.x + other,
            y: self.y + other,
        };
    }
}

impl Sub for Vector2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Sub<f32> for Vector2 {
    type Output = Self;
    fn sub(self, other: f32) -> Self::Output {
        Self {
            x: self.x - other,
            y: self.y - other,
        }
    }
}

impl SubAssign for Vector2 {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
        };
    }
}

impl SubAssign<f32> for Vector2 {
    fn sub_assign(&mut self, other: f32) {
        *self = Self {
            x: self.x - other,
            y: self.y - other,
        };
    }
}

impl Mul for Vector2 {
    type Output = Self;
    fn mul(self, other: Self) -> Self::Output {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl Mul<f32> for Vector2 {
    type Output = Self;
    fn mul(self, other: f32) -> Self::Output {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl MulAssign for Vector2 {
    fn mul_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x * other.x,
            y: self.y * other.y,
        };
    }
}

impl MulAssign<f32> for Vector2 {
    fn mul_assign(&mut self, other: f32) {
        *self = Self {
            x: self.x * other,
            y: self.y * other,
        };
    }
}

impl Div for Vector2 {
    type Output = Self;
    fn div(self, other: Self) -> Self::Output {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}

impl Div<f32> for Vector2 {
    type Output = Self;
    fn div(self, other: f32) -> Self::Output {
        Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl DivAssign for Vector2 {
    fn div_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x / other.x,
            y: self.y / other.y,
        };
    }
}

impl DivAssign<f32> for Vector2 {
    fn div_assign(&mut self, other: f32) {
        *self = Self {
            x: self.x / other,
            y: self.y / other,
        };
    }
}

impl Neg for Vector2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

// endregion Add

impl Vector2 {
    pub fn new() -> Self {
        Self { x: 0., y: 0. }
    }

    // pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    //     wgpu::VertexBufferLayout {
    //         array_stride: std::mem::size_of::<Vector2>() as wgpu::BufferAddress,
    //         step_mode: wgpu::VertexStepMode::Vertex,
    //         attributes: &[
    //             wgpu::VertexAttribute {
    //                 offset: 0,
    //                 shader_location: 0,
    //                 format: wgpu::VertexFormat::Float32x2,
    //             },
    //         ],
    //     }
    // }

    pub fn magnitude(&self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn angle(&self) -> f32 {
        self.y.atan2(self.x)
    }

    pub fn angle_to(&self, other: Vector2) -> f32 {
        (*self - other).angle()
    }

    pub fn distance(&self, other: Self) -> f32 {
        (other.x - self.x).hypot(other.y - self.y).abs()
    }

    pub fn normalized(self) -> Self {
        self / self.magnitude()
    }

    pub fn normalize(&mut self) {
        *self /= self.magnitude();
    }

    pub fn forward(&mut self, distance: f32) {
        *self += self.normalized() * distance;
    }

    pub fn onward(&mut self, distance: f32, angle: f32) {
        *self += Vector2 {
            x: angle.cos(),
            y: angle.sin(),
        } * distance;
    }

    pub fn set_angle(&mut self, angle: f32) {
        let mag: f32 = self.magnitude();
        self.x = angle.cos();
        self.y = angle.sin();
        *self *= mag;
    }

    pub fn rotate(&mut self, angle: f32) {
        let new_angle: f32 = self.angle() + angle;
        self.set_angle(new_angle);
    }

    pub fn face(&mut self, other: Self) {
        let angle = (other.x - self.x).hypot(other.y - self.y);
        self.set_angle(angle);
    }
}
