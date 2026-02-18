//! [`PhysicsWorld`] — rapier2d 0.22 integration and collision query interface.

use glam::Vec2;
use rapier2d::prelude::*;

/// The physics simulation world. Stored as a resource in `GurWorld`.
pub struct PhysicsWorld {
    pub bodies:         RigidBodySet,
    pub colliders:      ColliderSet,
    broad_phase:        DefaultBroadPhase,
    narrow_phase:       NarrowPhase,
    islands:            IslandManager,
    impulse_joints:     ImpulseJointSet,
    multibody_joints:   MultibodyJointSet,
    ccd_solver:         CCDSolver,
    gravity:            rapier2d::math::Vector<f32>,
    pub timestep:       f32,
    integration_params: IntegrationParameters,
    pipeline:           PhysicsPipeline,
    pub query_pipeline: QueryPipeline,
}

impl PhysicsWorld {
    /// Create a new physics world.
    pub fn new(gravity: Vec2, timestep: f32) -> Self {
        let mut integration_params = IntegrationParameters::default();
        integration_params.dt = timestep;

        Self {
            bodies:           RigidBodySet::new(),
            colliders:        ColliderSet::new(),
            broad_phase:      DefaultBroadPhase::new(),
            narrow_phase:     NarrowPhase::new(),
            islands:          IslandManager::new(),
            impulse_joints:   ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver:       CCDSolver::new(),
            gravity:          vector![gravity.x, gravity.y],
            timestep,
            integration_params,
            pipeline:         PhysicsPipeline::new(),
            query_pipeline:   QueryPipeline::new(),
        }
    }

    /// Advance the simulation by one fixed step.
    pub fn step(&mut self) {
        self.pipeline.step(
            &self.gravity,
            &self.integration_params,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    /// Cast a ray and return the closest hit collider handle.
    pub fn cast_ray(&self, origin: Vec2, direction: Vec2, max_distance: f32) -> Option<(ColliderHandle, f32)> {
        let ray = Ray::new(point![origin.x, origin.y], vector![direction.x, direction.y]);
        let filter = QueryFilter::default();
        self.query_pipeline
            .cast_ray(&self.bodies, &self.colliders, &ray, max_distance, true, filter)
    }

    /// Fast AABB overlap test (no rapier involved).
    pub fn aabb_overlap(a_center: Vec2, a_half: Vec2, b_center: Vec2, b_half: Vec2) -> bool {
        let dx = (a_center.x - b_center.x).abs();
        let dy = (a_center.y - b_center.y).abs();
        dx < (a_half.x + b_half.x) && dy < (a_half.y + b_half.y)
    }
}
