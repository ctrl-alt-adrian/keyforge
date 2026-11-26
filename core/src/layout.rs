use crate::{Error, Result, domain::KeyPosition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyGeometry {
    pub position: KeyPosition,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Degrees clockwise in a screen coordinate system, about the metadata pivot.
    pub rotation: f32,
    pub rotation_origin: Point,
}
impl KeyGeometry {
    fn rotate(&self, p: Point, angle: f32) -> Point {
        let (sin, cos) = angle.to_radians().sin_cos();
        let x = p.x - self.rotation_origin.x;
        let y = p.y - self.rotation_origin.y;
        Point {
            x: self.rotation_origin.x + x * cos - y * sin,
            y: self.rotation_origin.y + x * sin + y * cos,
        }
    }
    pub fn corners(&self, inset: f32) -> [Point; 4] {
        [
            Point {
                x: self.x + inset,
                y: self.y + inset,
            },
            Point {
                x: self.x + self.width - inset,
                y: self.y + inset,
            },
            Point {
                x: self.x + self.width - inset,
                y: self.y + self.height - inset,
            },
            Point {
                x: self.x + inset,
                y: self.y + self.height - inset,
            },
        ]
        .map(|p| self.rotate(p, self.rotation))
    }
    pub fn center(&self) -> Point {
        self.rotate(
            Point {
                x: self.x + self.width / 2.,
                y: self.y + self.height / 2.,
            },
            self.rotation,
        )
    }
    pub fn contains(&self, point: Point) -> bool {
        let p = self.rotate(point, -self.rotation);
        let inset = 0.045;
        p.x >= self.x + inset
            && p.x <= self.x + self.width - inset
            && p.y >= self.y + inset
            && p.y <= self.y + self.height - inset
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyboard {
    pub name: String,
    pub source: String,
    pub keys: Vec<KeyGeometry>,
}
impl Keyboard {
    pub fn check(&self) -> Result<()> {
        for (i, k) in self.keys.iter().enumerate() {
            if k.position.0 != i
                || ![
                    k.x,
                    k.y,
                    k.width,
                    k.height,
                    k.rotation,
                    k.rotation_origin.x,
                    k.rotation_origin.y,
                ]
                .iter()
                .all(|v| v.is_finite())
                || k.width <= 0.
                || k.height <= 0.
            {
                return Err(Error::InvalidDocument("Invalid key geometry/order".into()));
            }
        }
        Ok(())
    }
    pub fn bounds(&self) -> (Point, Point) {
        let mut min = Point {
            x: f32::INFINITY,
            y: f32::INFINITY,
        };
        let mut max = Point {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
        };
        for p in self.keys.iter().flat_map(|k| k.corners(0.)) {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
        }
        (min, max)
    }
    pub fn hit_test(&self, p: Point) -> Option<KeyPosition> {
        self.keys
            .iter()
            .rev()
            .find(|k| k.contains(p))
            .map(|k| k.position)
    }
    pub fn navigate(&self, from: KeyPosition, dx: f32, dy: f32) -> KeyPosition {
        let Some(origin) = self.keys.get(from.0).map(KeyGeometry::center) else {
            return KeyPosition(0);
        };
        self.keys
            .iter()
            .filter_map(|k| {
                let p = k.center();
                let x = p.x - origin.x;
                let y = p.y - origin.y;
                let along = x * dx + y * dy;
                let across = (x * dy - y * dx).abs();
                (along > 0.01).then_some((k.position, along + across * 3.))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(p, _)| p)
            .unwrap_or(from)
    }
}
/// Canvas camera is plain geometry data. Frameworks own its transient instance.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub zoom: f32,
    pub pan: Point,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.,
            pan: Point::default(),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub scale: f32,
    pub offset: Point,
}
impl Camera {
    pub fn transform(&self, keyboard: &Keyboard, width: f32, height: f32) -> Transform {
        let (min, max) = keyboard.bounds();
        let w = max.x - min.x;
        let h = max.y - min.y;
        let fit = ((width - 40.).max(1.) / w).min((height - 40.).max(1.) / h);
        let scale = (fit * self.zoom).max(0.01);
        Transform {
            scale,
            offset: Point {
                x: (width - w * scale) / 2. - min.x * scale + self.pan.x,
                y: (height - h * scale) / 2. - min.y * scale + self.pan.y,
            },
        }
    }
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.3, 4.);
    }
}
impl Transform {
    pub fn screen(&self, p: Point) -> Point {
        Point {
            x: p.x * self.scale + self.offset.x,
            y: p.y * self.scale + self.offset.y,
        }
    }
    pub fn world(&self, p: Point) -> Point {
        Point {
            x: (p.x - self.offset.x) / self.scale,
            y: (p.y - self.offset.y) / self.scale,
        }
    }
}
