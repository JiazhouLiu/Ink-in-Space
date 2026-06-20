use na::{Quaternion, Vector2, Vector3};
use nalgebra as na;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WritingPlane {
    /// XZ Plane
    Horizontal,
    /// XY Plane
    Vertical,
    /// Detect
    Auto,
}

impl WritingPlane {
    fn detect(pos: &[Vector3<f32>]) -> WritingPlane {
        let n = pos.len() as f32;
        let mean = pos.iter().fold(Vector3::zeros(), |a, &p| a + p) / n;
        let var = |val: fn(&Vector3<f32>) -> f32| pos.iter().map(|p| (val(p) - val(&mean)).powi(2)).sum::<f32>() / n;
        let vx = var(|p| p.x);
        let vy = var(|p| p.y);
        let vz = var(|p| p.z);

        if vx + vy >= vx + vz {
            WritingPlane::Vertical
        } else {
            WritingPlane::Horizontal
        }
    }
}

#[derive(Debug)]
pub struct Trajectory {
    position: Vec<Vector3<f32>>,
    rotation: Vec<Quaternion<f32>>,
    strokes: Vec<usize>,
}

impl Trajectory {
    pub fn new() -> Self {
        Trajectory {
            position: Vec::with_capacity(1000),
            rotation: Vec::with_capacity(1000),
            strokes: vec![0],
        }
    }

    pub fn push(&mut self, position: Vector3<f32>, rotation: Quaternion<f32>) {
        self.position.push(position);
        self.rotation.push(rotation);
    }

    pub fn start_stroke(&mut self) {
        self.strokes.push(self.position.len());
    }

    pub fn undo(&mut self) {
        if let Some(last) = self.strokes.pop() {
            self.position.truncate(last);
            self.rotation.truncate(last);
        }
    }

    pub fn reset(&mut self) {
        self.position.clear();
        self.rotation.clear();
        self.strokes.clear();
    }

    // Interpolate points along each stroke for smoothness
    fn interp(&self, step: f32) -> (Vec<Vector3<f32>>, usize) {
        let mut fc = 0;
        let mut res = Vec::new();

        for stroke in 0..self.strokes.len() {
            let start = self.strokes[stroke];
            let end = self.strokes.get(stroke + 1).copied().unwrap_or(self.position.len());

            if start >= end {
                continue;
            }

            let mut prev = self.position[start];
            res.push(prev);

            for p in &self.position[start + 1..end] {
                let delta = *p - prev;
                let dist = delta.norm();

                if dist > step {
                    let steps = (dist / step).ceil() as usize;
                    for i in 1..=steps {
                        res.push(prev + delta * (i as f32 / steps as f32));
                    }
                } else {
                    res.push(*p);
                }

                prev = *p;
            }

            if stroke == 0 {
                fc = res.len();
            }
        }

        (res, fc)
    }

    pub fn normalise(&self, rotate_degrees: f32, plane: WritingPlane) -> (Vec<Vector2<f32>>, usize) {
        let step = 0.00001;
        let (pos, fc) = self.interp(step);

        if pos.is_empty() {
            return (vec![], 0);
        }

        let plane = match plane {
            WritingPlane::Auto => WritingPlane::detect(&pos),
            plane => plane,
        };

        let mut proj: Vec<Vector2<f32>> = pos
            .iter()
            .map(|v| match plane {
                WritingPlane::Horizontal => Vector2::new(v.x, v.z),
                WritingPlane::Vertical => Vector2::new(v.x, v.y),
                WritingPlane::Auto => unreachable!(),
            })
            .collect();

        let mut min = proj[0];
        let mut max = proj[0];
        for p in &proj {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
        }

        let center = (min + max) * 0.5;
        let size = max - min;
        let scale = 2.0 / size.x.max(size.y);

        for p in &mut proj {
            *p = (*p - center) * scale;
        }

        // Rotate each point around the origin by rotate_degrees
        let angle = rotate_degrees.to_radians();
        let (sin, cos) = angle.sin_cos();
        for p in &mut proj {
            let x = p.x * cos - p.y * sin;
            let y = p.x * sin + p.y * cos;
            *p = Vector2::new(x, y);
        }

        (proj, fc)
    }
}

impl Default for Trajectory {
    fn default() -> Self {
        Trajectory::new()
    }
}
