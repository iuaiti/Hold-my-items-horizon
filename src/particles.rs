//! First-person particle system ported from `Particle.java`, `ParticleManager.java`,
//! `ParticleRenderManager.java`, and `patricles/enums/*`.
//!
//! Renders billboard quads in `ITEM` or `SCREEN` space directly onto Horizon's
//! `BlockEntityDrawContext` during `Hooks::render_hand`.

use mod_api::{BlockEntityDrawContext, Hand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderSpace {
    Item,
    Screen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleLifecycle {
    Scale,
    Opacity,
    Spawn,
    Keyframe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleRenderType {
    Cutout,
    CutoutLit,
    Additive,
    Translucent,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub gravity: bool,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub rx: f64,
    pub ry: f64,
    pub rz: f64,
    pub drx: f64,
    pub dry: f64,
    pub drz: f64,
    pub scale: f64,
    pub max_scale: f64,
    pub alpha: f64,
    pub lifetime: f64,
    pub age: f64,
    pub is_birth: bool,
    pub dead: bool,
    pub sprite: String,
    pub keyframes: Vec<String>,
    pub keyframe_index: usize,
    pub prev_player_age: i32,
    pub space: RenderSpace,
    pub hand: Hand,
    pub lifecycle: ParticleLifecycle,
    pub render_type: ParticleRenderType,
}

impl Particle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gravity: bool,
        pos: [f64; 3],
        vel: [f64; 3],
        rot: [f64; 3],
        drot: [f64; 3],
        max_scale: f64,
        sprite: impl Into<String>,
        space: RenderSpace,
        hand: Hand,
        lifecycle: ParticleLifecycle,
        render_type: ParticleRenderType,
        lifetime: f64,
        alpha: f64,
    ) -> Self {
        Self {
            gravity,
            x: pos[0],
            y: pos[1],
            z: pos[2],
            dx: vel[0],
            dy: vel[1],
            dz: vel[2],
            rx: rot[0],
            ry: rot[1],
            rz: rot[2],
            drx: drot[0],
            dry: drot[1],
            drz: drot[2],
            scale: if lifecycle == ParticleLifecycle::Scale {
                0.0
            } else {
                max_scale
            },
            max_scale,
            alpha,
            lifetime,
            age: 0.0,
            is_birth: true,
            dead: false,
            sprite: sprite.into(),
            keyframes: Vec::new(),
            keyframe_index: 0,
            prev_player_age: -1,
            space,
            hand,
            lifecycle,
            render_type,
        }
    }

    pub fn with_keyframes(mut self, frames: Vec<String>) -> Self {
        self.keyframes = frames;
        self
    }

    /// Mirrors `Particle.tick()` from HMI 5.1.1.
    pub fn tick(&mut self, delta_time: f32, player_age: i32) {
        if self.dead {
            return;
        }
        let dt30 = (delta_time as f64) * 30.0;
        self.lifetime = (self.lifetime - 0.1 * dt30).clamp(0.0, 9999.0);
        self.age += 0.1 * dt30;

        self.x += self.dx * dt30;
        self.y += self.dy * dt30;
        self.z += self.dz * dt30;

        let drag = 0.9_f64.powf(dt30);
        self.dx *= drag;
        self.dy *= drag;
        self.dz *= drag;

        if self.gravity {
            self.dy -= 0.01 * dt30;
        }

        self.rx += self.drx * dt30;
        self.ry += self.dry * dt30;
        self.rz += self.drz * dt30;

        if !self.keyframes.is_empty()
            && self.keyframe_index < self.keyframes.len()
            && player_age % 4 == 0
            && player_age != self.prev_player_age
        {
            self.sprite = self.keyframes[self.keyframe_index].clone();
            self.keyframe_index += 1;
        }

        match self.lifecycle {
            ParticleLifecycle::Scale => {
                if !self.is_birth && self.lifetime <= 0.0 {
                    self.scale -= 0.03 * dt30;
                    if self.scale <= 0.0 {
                        self.dead = true;
                    }
                } else {
                    self.scale += 0.07 * dt30;
                    if self.scale >= self.max_scale {
                        self.scale = self.max_scale;
                        self.is_birth = false;
                    }
                }
            }
            ParticleLifecycle::Opacity => {
                if !self.is_birth && self.lifetime <= 0.0 {
                    self.alpha -= 10.0 * dt30;
                    if self.alpha <= 0.0 {
                        self.alpha = 0.0;
                        self.dead = true;
                    }
                } else {
                    self.scale = self.max_scale;
                    self.is_birth = false;
                    self.alpha = 255.0;
                }
            }
            ParticleLifecycle::Spawn => {
                if !self.is_birth && self.lifetime <= 0.0 {
                    self.dead = true;
                } else {
                    self.scale = self.max_scale;
                    self.is_birth = false;
                }
            }
            ParticleLifecycle::Keyframe => {
                if !self.is_birth && self.keyframe_index >= self.keyframes.len() {
                    self.dead = true;
                } else {
                    self.scale = self.max_scale;
                    self.is_birth = false;
                }
            }
        }

        self.prev_player_age = player_age;
    }

    /// Renders this particle as a textured quad on Horizon's `BlockEntityDrawContext`.
    pub fn render(&self, draw: &mut dyn BlockEntityDrawContext) {
        if self.dead || self.scale <= 0.001 || self.alpha <= 1.0 {
            return;
        }

        let prev_light = draw.light();
        if matches!(
            self.render_type,
            ParticleRenderType::CutoutLit | ParticleRenderType::Additive
        ) {
            // Full bright light (sky 15, block 15) matching LightmapTextureManager.pack(15, 15)
            draw.set_light((15 << 4) | (15 << 20));
        }

        draw.push();
        draw.translate([self.x as f32, self.y as f32, self.z as f32]);
        let s = self.scale as f32;
        draw.scale([s, s, s]);
        if self.rx.abs() > 1e-4 {
            draw.rotate_about([1.0, 0.0, 0.0], self.rx as f32, [0.0, 0.0, 0.0]);
        }
        if self.ry.abs() > 1e-4 {
            draw.rotate_about([0.0, 1.0, 0.0], self.ry as f32, [0.0, 0.0, 0.0]);
        }
        if self.rz.abs() > 1e-4 {
            draw.rotate_about([0.0, 0.0, 1.0], self.rz as f32, [0.0, 0.0, 0.0]);
        }

        let half = 0.25_f32;
        let corners = [
            [-half, half, 0.0],
            [-half, -half, 0.0],
            [half, -half, 0.0],
            [half, half, 0.0],
        ];
        // Scale RGB slightly by alpha for additive/translucent fade
        let factor = (self.alpha / 255.0).clamp(0.0, 1.0);
        let c = (255.0 * factor) as u32;
        let tint = (c << 16) | (c << 8) | c;
        let _ = draw.quad(&self.sprite, corners, [0.0, 0.0, 16.0, 16.0], tint);
        draw.pop();

        draw.set_light(prev_light);
    }
}

/// Ticks and draws all particles belonging to `space` and `hand`, pruning dead ones.
pub fn draw_particles(
    particles: &mut Vec<Particle>,
    draw: &mut dyn BlockEntityDrawContext,
    space: RenderSpace,
    hand: Hand,
    delta_time: f32,
    player_age: i32,
) {
    for p in particles.iter_mut() {
        if p.space == space && p.hand == hand {
            p.tick(delta_time, player_age);
            p.render(draw);
        }
    }
    particles.retain(|p| !p.dead);
    // Cap particle pool to prevent runaway memory growth
    if particles.len() > 256 {
        let excess = particles.len() - 256;
        particles.drain(0..excess);
    }
}
