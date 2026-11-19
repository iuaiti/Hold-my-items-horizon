//! Matrix stack (`PoseStack`) and math utilities ported from `M.java`.
//!
//! `PoseStack` mirrors Minecraft's `MatrixStack` (`class_4587`) using `glam::Mat4`,
//! and provides conversion to Horizon's `HandRender` translation/rotation/scale
//! as well as direct replay onto `mod_api::BlockEntityDrawContext`.

use glam::{EulerRot, Mat4, Quat, Vec3};
use mod_api::BlockEntityDrawContext;

pub const PI: f64 = std::f64::consts::PI;

/// Recorded affine operations so a pose can both be evaluated as a `Mat4`
/// and replayed cleanly onto Horizon's `BlockEntityDrawContext`.
#[derive(Clone, Debug)]
pub enum PoseOp {
    Translate([f32; 3]),
    RotateAbout {
        axis: [f32; 3],
        degrees: f32,
        pivot: [f32; 3],
    },
    Scale([f32; 3]),
}

/// First-person pose matrix stack matching `M.java` (`MatrixStack` wrapper).
#[derive(Clone, Debug)]
pub struct PoseStack {
    stack: Vec<Mat4>,
    ops: Vec<PoseOp>,
}

impl Default for PoseStack {
    fn default() -> Self {
        Self::new()
    }
}

impl PoseStack {
    pub fn new() -> Self {
        Self {
            stack: vec![Mat4::IDENTITY],
            ops: Vec::with_capacity(32),
        }
    }

    pub fn push(&mut self) {
        let top = *self.stack.last().unwrap_or(&Mat4::IDENTITY);
        self.stack.push(top);
    }

    pub fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    #[inline]
    pub fn current(&self) -> Mat4 {
        *self.stack.last().unwrap_or(&Mat4::IDENTITY)
    }

    #[inline]
    fn current_mut(&mut self) -> &mut Mat4 {
        self.stack.last_mut().expect("PoseStack never empty")
    }

    #[inline]
    pub fn translate(&mut self, x: f64, y: f64, z: f64) {
        let v = [x as f32, y as f32, z as f32];
        *self.current_mut() *= Mat4::from_translation(Vec3::from_array(v));
        self.ops.push(PoseOp::Translate(v));
    }

    #[inline]
    pub fn move_x(&mut self, amount: f64) {
        self.translate(amount, 0.0, 0.0);
    }

    #[inline]
    pub fn move_y(&mut self, amount: f64) {
        self.translate(0.0, amount, 0.0);
    }

    #[inline]
    pub fn move_z(&mut self, amount: f64) {
        self.translate(0.0, 0.0, amount);
    }

    #[inline]
    pub fn scale(&mut self, x: f64, y: f64, z: f64) {
        let v = [x as f32, y as f32, z as f32];
        *self.current_mut() *= Mat4::from_scale(Vec3::from_array(v));
        self.ops.push(PoseOp::Scale(v));
    }

    #[inline]
    pub fn rotate_x(&mut self, degrees: f64) {
        self.rotate_axis([1.0, 0.0, 0.0], degrees as f32, [0.0, 0.0, 0.0]);
    }

    #[inline]
    pub fn rotate_y(&mut self, degrees: f64) {
        self.rotate_axis([0.0, 1.0, 0.0], degrees as f32, [0.0, 0.0, 0.0]);
    }

    #[inline]
    pub fn rotate_z(&mut self, degrees: f64) {
        self.rotate_axis([0.0, 0.0, 1.0], degrees as f32, [0.0, 0.0, 0.0]);
    }

    #[inline]
    pub fn rotate_x_pivot(&mut self, degrees: f64, px: f64, py: f64, pz: f64) {
        self.rotate_axis([1.0, 0.0, 0.0], degrees as f32, [px as f32, py as f32, pz as f32]);
    }

    #[inline]
    pub fn rotate_y_pivot(&mut self, degrees: f64, px: f64, py: f64, pz: f64) {
        self.rotate_axis([0.0, 1.0, 0.0], degrees as f32, [px as f32, py as f32, pz as f32]);
    }

    #[inline]
    pub fn rotate_z_pivot(&mut self, degrees: f64, px: f64, py: f64, pz: f64) {
        self.rotate_axis([0.0, 0.0, 1.0], degrees as f32, [px as f32, py as f32, pz as f32]);
    }

    pub fn rotate_axis(&mut self, axis: [f32; 3], degrees: f32, pivot: [f32; 3]) {
        if !degrees.is_finite() || degrees.abs() < 1e-6 {
            return;
        }
        let rad = degrees.to_radians();
        let q = Quat::from_axis_angle(Vec3::from_array(axis).normalize_or_zero(), rad);
        let p = Vec3::from_array(pivot);
        let m = if p.length_squared() > 0.0 {
            Mat4::from_translation(p) * Mat4::from_quat(q) * Mat4::from_translation(-p)
        } else {
            Mat4::from_quat(q)
        };
        *self.current_mut() *= m;
        self.ops.push(PoseOp::RotateAbout {
            axis,
            degrees,
            pivot,
        });
    }

    /// Shear transform matching `M.shear(matrices, shearX, shearY, shearZ)`.
    pub fn shear(&mut self, shear_x: f64, shear_y: f64, shear_z: f64) {
        let sx = shear_x as f32;
        let sy = shear_y as f32;
        let sz = shear_z as f32;
        let m = Mat4::from_cols_array(&[
            1.0, sx, sx, 0.0,
            sy, 1.0, sy, 0.0,
            sz, sz, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        *self.current_mut() *= m;
    }

    /// Decomposes the top matrix into `(translation, [pitch_deg, yaw_deg, roll_deg], uniform_scale)`
    /// for applying to Horizon's `HandRender`.
    pub fn decompose_for_hand_render(&self) -> ([f32; 3], [f32; 3], f32) {
        let mat = self.current();
        let (scale_vec, rot_quat, trans_vec) = mat.to_scale_rotation_translation();
        let (yaw, pitch, roll) = rot_quat.to_euler(EulerRot::YXZ);
        let uniform_scale = ((scale_vec.x + scale_vec.y + scale_vec.z) / 3.0).clamp(0.05, 4.0);
        let safe_t = [
            if trans_vec.x.is_finite() { trans_vec.x } else { 0.0 },
            if trans_vec.y.is_finite() { trans_vec.y } else { 0.0 },
            if trans_vec.z.is_finite() { trans_vec.z } else { 0.0 },
        ];
        let safe_r = [
            if pitch.is_finite() { pitch.to_degrees() } else { 0.0 },
            if yaw.is_finite() { yaw.to_degrees() } else { 0.0 },
            if roll.is_finite() { roll.to_degrees() } else { 0.0 },
        ];
        (
            safe_t,
            safe_r,
            if uniform_scale.is_finite() { uniform_scale } else { 1.0 },
        )
    }

    /// Replays all recorded affine operations onto a `BlockEntityDrawContext`.
    pub fn apply_to_draw(&self, draw: &mut dyn BlockEntityDrawContext) {
        for op in &self.ops {
            match *op {
                PoseOp::Translate(by) => draw.translate(by),
                PoseOp::RotateAbout {
                    axis,
                    degrees,
                    pivot,
                } => draw.rotate_about(axis, degrees, pivot),
                PoseOp::Scale(by) => draw.scale(by),
            }
        }
    }
}

#[inline]
pub fn clamp(a: f64, min: f64, max: f64) -> f64 {
    a.clamp(min, max)
}

#[inline]
pub fn lerp(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}

#[inline]
pub fn sin(a: f64) -> f64 {
    a.sin()
}

#[inline]
pub fn cos(a: f64) -> f64 {
    a.cos()
}

#[inline]
pub fn abs(a: f64) -> f64 {
    a.abs()
}

#[inline]
pub fn pow(a: f64, b: f64) -> f64 {
    a.powf(b)
}
