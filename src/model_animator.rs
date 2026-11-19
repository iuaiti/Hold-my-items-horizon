//! Per-element model part animator ported from `com.holdmylua.source.model.*`
//! (`ModelPartAnimator`, `PoseX/Y/Z`, `RotationX/Y/Z`, `ScalePose`).

use crate::math::PoseStack;

#[derive(Clone, Copy, Debug)]
pub enum ModelPose {
    MoveX { index: usize, amount: f32 },
    MoveY { index: usize, amount: f32 },
    MoveZ { index: usize, amount: f32 },
    RotateX { index: usize, degrees: f32, pivot: [f32; 3] },
    RotateY { index: usize, degrees: f32, pivot: [f32; 3] },
    RotateZ { index: usize, degrees: f32, pivot: [f32; 3] },
    Scale { index: usize, factor: [f32; 3] },
}

impl ModelPose {
    pub fn index(&self) -> usize {
        match *self {
            Self::MoveX { index, .. }
            | Self::MoveY { index, .. }
            | Self::MoveZ { index, .. }
            | Self::RotateX { index, .. }
            | Self::RotateY { index, .. }
            | Self::RotateZ { index, .. }
            | Self::Scale { index, .. } => index,
        }
    }

    pub fn apply(&self, stack: &mut PoseStack) {
        match *self {
            Self::MoveX { amount, .. } => stack.move_x(amount as f64),
            Self::MoveY { amount, .. } => stack.move_y(amount as f64),
            Self::MoveZ { amount, .. } => stack.move_z(amount as f64),
            Self::RotateX { degrees, pivot, .. } => {
                stack.rotate_axis([1.0, 0.0, 0.0], degrees, pivot)
            }
            Self::RotateY { degrees, pivot, .. } => {
                stack.rotate_axis([0.0, 1.0, 0.0], degrees, pivot)
            }
            Self::RotateZ { degrees, pivot, .. } => {
                stack.rotate_axis([0.0, 0.0, 1.0], degrees, pivot)
            }
            Self::Scale { factor, .. } => {
                stack.scale(factor[0] as f64, factor[1] as f64, factor[2] as f64)
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ModelPartAnimator {
    poses: Vec<ModelPose>,
}

impl ModelPartAnimator {
    pub fn move_x(&mut self, start: usize, end: usize, amount: f32) {
        for index in start..=end {
            self.poses.push(ModelPose::MoveX { index, amount });
        }
    }

    pub fn move_y(&mut self, start: usize, end: usize, amount: f32) {
        for index in start..=end {
            self.poses.push(ModelPose::MoveY { index, amount });
        }
    }

    pub fn move_z(&mut self, start: usize, end: usize, amount: f32) {
        for index in start..=end {
            self.poses.push(ModelPose::MoveZ { index, amount });
        }
    }

    pub fn rotate_x(&mut self, start: usize, end: usize, degrees: f32, pivot: [f32; 3]) {
        for index in start..=end {
            self.poses.push(ModelPose::RotateX {
                index,
                degrees,
                pivot,
            });
        }
    }

    pub fn rotate_y(&mut self, start: usize, end: usize, degrees: f32, pivot: [f32; 3]) {
        for index in start..=end {
            self.poses.push(ModelPose::RotateY {
                index,
                degrees,
                pivot,
            });
        }
    }

    pub fn rotate_z(&mut self, start: usize, end: usize, degrees: f32, pivot: [f32; 3]) {
        for index in start..=end {
            self.poses.push(ModelPose::RotateZ {
                index,
                degrees,
                pivot,
            });
        }
    }

    pub fn scale(&mut self, start: usize, end: usize, x: f32, y: f32, z: f32) {
        for index in start..=end {
            self.poses.push(ModelPose::Scale {
                index,
                factor: [x, y, z],
            });
        }
    }

    /// Applies all recorded poses for `index` in reverse insertion order (matching Java).
    pub fn apply_poses(&self, index: usize, stack: &mut PoseStack) {
        for pose in self.poses.iter().rev() {
            if pose.index() == index {
                pose.apply(stack);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.poses.is_empty()
    }

    pub fn clear(&mut self) {
        self.poses.clear();
    }
}
