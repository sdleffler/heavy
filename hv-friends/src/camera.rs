//! A 2D camera algorithm intended for top-down usage (but usable for pretty much anything 2D) which
//! smoothly scales, rotates, and translates the world so as to try to best fit the subject's view
//! to nearby "focus points" registered with it.
//!
//! Foci have both a centroid and a collider. If the subject is inside the collider of some given
//! focus, then that focus is considered to be a "main" focus. "Main" foci are weighted higher when
//! interpolating between translations.
//!
//! The camera's *translation* (not rotation or scale) are interpolated using Shepard's
//! interpolation (inverse distance weighting) between foci. The translation again is then averaged
//! against the subject's translation, to ensure the subject is always at least somewhat centered.
//! Rotation and scale are not interpolated but rather determined by the orientation of the nearest
//! foci and the smallest scale which can fit the foci's entire collider within the screen
//! dimensions passed to the camera. With respect to these two parameters, a smooth lerp'd
//! transition occurs when moving between the Voronoi cells of two foci, and they are otherwise not
//! interpolated; when the subject stays within a single foci's Voronoi cell, the rotation and scale
//! will smoothly change and then stay until the subject moves closer to another foci.
//!
//! # Additional [`Focus`] parameters
//!
//! Besides the collider and local transform of the collider, foci have a few additional parameters:
//!
//! - `override_scale`: Allows for overriding the calculated scale of the focus. Normally, the scale
//!   is calculated to be the smallest possible scale which would put the focus's collider's
//!   bounding box entirely on-screen. The `override_scale` parameter allows you to change that to
//!   whatever constant value you like, whether you're looking for a claustrophobic zoomed-in effect
//!   or a bird's eye view.
//! - `weight_against_subject`: Once the closest focus is chosen and the interpolated translation is
//!   calculated with respect to all foci, the final translation is blended with the subject's
//!   translation in order to ensure that the subject is more centered (if wanted.) The default
//!   value for this should be more than enough for most purposes, but if the focus is a boss arena
//!   then you might want to set the weight to `1.0` to ensure the subject cannot cause the camera
//!   to move, or if the focus is a long passage, then perhaps you would prefer the camera move
//!   entirely with the subject (in which case set the `weight_against_subject` parameter to `0.0`).
//! - `center`: By default the focus's centerpoint, used for interpolation and finding the camera
//!   translation, is set to be the result of translating the point at the origin by the collider's
//!   transform. This default value can be overriden by setting the `center` parameter.
//! - `orientation`: Allows for setting an orientation for the area. The default value is zero, but
//!   if you want the subject to view the place at any other angle, this parameter sets the
//!   resulting orientation of the calculated transform whenever this focus is the "hot focus" that
//!   the subject is currently "inside".
use crate::{math::*, parry2d::shape::SharedShape};

use hv_core::mlua::prelude::*;
use thunderdome::{Arena, Index};

#[derive(Clone)]
pub struct Focus {
    pub collider: SharedShape,
    pub collider_tx: Isometry2<f32>,
    pub override_scale: Option<f32>,
    pub weight_against_subject: f32,
    pub center: Point2<f32>,
    pub orientation: f32,
}

impl Focus {
    pub fn new(collider: SharedShape, collider_tx: Isometry2<f32>) -> Self {
        Self {
            collider,
            collider_tx,
            override_scale: None,
            weight_against_subject: 0.8,
            center: collider_tx.translation.vector.into(),
            orientation: 0.,
        }
    }

    pub fn override_scale(mut self, override_scale: Option<f32>) -> Self {
        self.override_scale = override_scale;
        self
    }

    pub fn weight_against_subject(mut self, weight: f32) -> Self {
        self.weight_against_subject = weight;
        self
    }

    pub fn center(mut self, center: Point2<f32>) -> Self {
        self.center = center;
        self
    }

    pub fn orientation(mut self, orientation: f32) -> Self {
        self.orientation = orientation;
        self
    }

    fn calculate_scale(&self, screen_dimensions: &Vector2<u32>) -> f32 {
        // A focus may choose to override the scale calculated from its collider's AABB. This is
        // usual in situations where you don't want the subject to see the entire focus, or in cases
        // where the area of the focus is so large that it wouldn't make sense to try and put the
        // entire thing on-screen.
        if let Some(overridden_scale) = self.override_scale {
            return overridden_scale;
        }

        let mut local_tx = self.collider_tx;
        // It shouldn't matter if we rotate the shape w.r.t. its center or w.r.t. the focus's center
        // because we don't care about the translation of the AABB, only its dimensions.
        local_tx.append_rotation_wrt_center_mut(&UnitComplex::new(self.orientation));
        let aabb = self.collider.compute_aabb(&local_tx);
        let focus_dimensions = aabb.extents();

        // We calculate the ratio of focus dimension to screen dimension, and then choose the
        // highest such ratio as our scale, so as to ensure that the entire area fits on-screen no
        // matter what.
        let scales = screen_dimensions
            .cast::<f32>()
            .component_div(&focus_dimensions);
        scales.min()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FocusIndex(Index);

impl<'lua> ToLua<'lua> for FocusIndex {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        LuaLightUserData(self.0.to_bits() as *mut _).to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for FocusIndex {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        LuaLightUserData::from_lua(lua_value, lua).map(|lud| Self(Index::from_bits(lud.0 as u64)))
    }
}

const TRANSITION_TIME_CONSTANT: f32 = 1.;

struct TransitionState {
    from_orientation: f32,
    from_scale: f32,
    to_orientation: f32,
    to_scale: f32,
    t: f32,
}

impl Default for TransitionState {
    fn default() -> Self {
        Self {
            from_orientation: 0.,
            from_scale: 1.,
            to_orientation: 0.,
            to_scale: 1.,

            // We are considered to be in the middle of a transition if `t` is less than the
            // transition time constant, so we set it to an obviously higher value here so that
            // we're never transitioning when we first initialize the camera.
            t: 2. * TRANSITION_TIME_CONSTANT,
        }
    }
}

impl TransitionState {
    fn new(current_orientation: f32, current_scale: f32) -> Self {
        Self {
            from_orientation: current_orientation,
            from_scale: current_scale,
            to_orientation: current_orientation,
            to_scale: current_scale,
            t: 0.,
        }
    }

    fn lerped_orientation(&self) -> f32 {
        (self.t / TRANSITION_TIME_CONSTANT)
            .min(1.0)
            .lerp(self.from_orientation, self.to_orientation)
    }

    fn lerped_scale(&self) -> f32 {
        (self.t / TRANSITION_TIME_CONSTANT)
            .min(1.0)
            .lerp(self.from_scale, self.to_scale)
    }

    fn is_in_flux(&self) -> bool {
        self.t < TRANSITION_TIME_CONSTANT
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CameraParameters {
    /// Screen dimensions; used for calculating scales.
    pub screen_dimensions: Vector2<u32>,
    /// The factor multiplied into the weights of "main" foci.
    pub main_foci_weight_factor: f32,
    /// The factor multiplied into the weights of "secondary" foci.
    pub secondary_foci_weight_factor: f32,
    /// The "power" parameter (p) used when interpolating foci positions via IDW.
    pub idw_power: f32,
    /// The exponential decay parameter controlling how fast the target transform approaches the
    /// focus transform.
    pub time_constant: f32,
}

impl CameraParameters {
    pub fn new(screen_dimensions: Vector2<u32>) -> Self {
        Self {
            screen_dimensions,
            main_foci_weight_factor: 1.0,
            secondary_foci_weight_factor: 0.5,
            idw_power: 2.5,
            time_constant: 1.,
        }
    }
}

pub struct Camera {
    /// The constant parameters.
    params: CameraParameters,
    /// The current state if transitioning from one foci to another.
    transition_state: TransitionState,
    /// Storage for all unique foci the camera knows about.
    foci: Arena<Focus>,
    /// The "hot" focus is the focus which the subject is currently closest to.
    hot_focus: Option<Index>,
    /// The current position of the subject. We don't care about orientation of the subject here
    /// because the orientation is determined by the main focus.
    subject_pos: Point2<f32>,
    /// The base scaling factor.
    base_scale: f32,
    /// The calculated transform, calculated from the position of the subject and the foci. This is
    /// the eventual destination of the target transform (if the subject stops moving.)
    calculated_tx: Similarity2<f32>,
    /// The target transform.
    target_tx: Similarity2<f32>,
    /// The current "world" camera transform.
    world_tx: Similarity2<f32>,
    /// The current "screen" transform; this is just the inverse of the world transform.
    screen_tx: Similarity2<f32>,
}

impl Camera {
    pub fn new(params: CameraParameters) -> Self {
        Self {
            params,
            transition_state: TransitionState::default(),
            foci: Arena::new(),
            hot_focus: None,
            subject_pos: Point2::origin(),
            base_scale: 1.,
            calculated_tx: Similarity2::identity(),
            target_tx: Similarity2::identity(),
            world_tx: Similarity2::identity(),
            screen_tx: Similarity2::identity(),
        }
    }

    pub fn subject_pos(&self) -> Point2<f32> {
        self.subject_pos
    }

    pub fn set_subject_pos(&mut self, subject_pos: Point2<f32>) {
        self.subject_pos = subject_pos;
    }

    pub fn scale(&self) -> f32 {
        self.base_scale
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.base_scale = scale;
    }

    pub fn insert_focus(&mut self, focus: Focus) -> FocusIndex {
        FocusIndex(self.foci.insert(focus))
    }

    pub fn remove_focus(&mut self, focus_index: FocusIndex) {
        self.foci.remove(focus_index.0);
    }

    pub fn clear_foci(&mut self) {
        self.foci.clear();
    }

    pub fn recalculate(&mut self) {
        let mut total_weighted_translations = Vector2::zeros();
        let mut total_weight = 0.;
        let mut closest_focus = None;
        let mut closest_focus_distance = f32::INFINITY;

        for (index, focus) in self.foci.iter() {
            let distance = na::distance(&focus.center, &self.subject_pos);

            if distance <= f32::EPSILON {
                closest_focus_distance = 0.;
                closest_focus = Some(index);
                break;
            } else {
                let is_main = focus
                    .collider
                    .contains_point(&focus.collider_tx, &self.subject_pos);

                // Only main focuses may be considered as the "hot focus".
                if distance < closest_focus_distance && is_main {
                    closest_focus_distance = distance;
                    closest_focus = Some(index);
                }

                let weight_factor = match is_main {
                    true => self.params.main_foci_weight_factor,
                    false => self.params.secondary_foci_weight_factor,
                };

                let weight = distance.powf(-self.params.idw_power) * weight_factor;
                total_weighted_translations += focus.center.coords * weight;
                total_weight += weight;
            }
        }

        // There are three cases to consider. In the first, we have a closest focus which is of zero
        // distance from the subject (highly, highly unlikely, but possible, and if we ignore it
        // we'll have problems, divide by zero problems.) In the second, we have no foci at all; and
        // in the third, the "normal" case, we calculate the weighted average of the foci's
        // translations.
        let interpolated_translation = match closest_focus {
            Some(closest) if closest_focus_distance == 0. => self.foci[closest].center.coords,
            _ if total_weight == 0. => self.subject_pos.coords,
            _ => total_weighted_translations / total_weight,
        };

        if closest_focus != self.hot_focus {
            self.transition_state = TransitionState::new(
                self.target_tx.isometry.rotation.angle(),
                self.target_tx.scaling(),
            );

            if let Some(hot_index) = self.hot_focus {
                self.transition_state.from_orientation = self.foci[hot_index].orientation;
                self.transition_state.from_scale =
                    self.foci[hot_index].calculate_scale(&self.params.screen_dimensions);
            }

            if let Some(closest_index) = closest_focus {
                self.transition_state.to_orientation = self.foci[closest_index].orientation;
                self.transition_state.to_scale =
                    self.foci[closest_index].calculate_scale(&self.params.screen_dimensions);
            }

            self.hot_focus = closest_focus;
        }

        // Whether the transition is in flux or not, we always are calculating the new/interpolated
        // translation.
        self.calculated_tx.isometry.translation.vector = interpolated_translation;

        let lerped_orientation = self.transition_state.lerped_orientation();
        let lerped_scale = self.transition_state.lerped_scale();

        self.calculated_tx.isometry.rotation = UnitComplex::new(lerped_orientation);
        self.calculated_tx.set_scaling(lerped_scale);
    }

    pub fn update(&mut self, dt: f32) {
        if self.transition_state.is_in_flux() {
            self.transition_state.t += dt;
        }

        self.recalculate();

        // Performing these lerp assignments creates an exponential decay which causes the
        // target transform to approach the calculated transform over time.
        self.target_tx.isometry = self
            .calculated_tx
            .isometry
            .lerp_slerp(&self.target_tx.isometry, self.params.time_constant * dt);

        self.target_tx.set_scaling(
            (self.params.time_constant * dt)
                .lerp(self.calculated_tx.scaling(), self.target_tx.scaling()),
        );

        // After all this effort calculating the smoothly moving target transform, we then create
        // the world transform using the target transform's rotation and scaling but a weighted
        // average of the subject's translation and the "hot focus"'s weight-against-subject
        // parameter. The higher the hot focus's weight against the subject, the less the subject's
        // translation will have an influence in the calculated world transform, and vice versa.
        // This is useful in the case where, say, a subject is going down a very small passage and
        // we want a claustrophobic impression; the small passage's camera focus can override its
        // scale to be very close-up, and set the weight against the subject to be zero, so that the
        // subject only sees their immediate surroundings. On the other hand if we are in a boss
        // arena, we probably don't want the camera to move at all; so we can set the weight against
        // the subject to be 1.0, which causes the focus to be the only factor in the calculated
        // translation.
        self.world_tx = Similarity2::identity();
        self.world_tx.append_translation_mut(&Translation2::from(
            -self.params.screen_dimensions.cast::<f32>() / 2.,
        ));
        self.world_tx
            .append_scaling_mut(self.target_tx.scaling() * self.base_scale);
        self.world_tx.append_translation_mut(&Translation2::from(
            self.subject_pos.coords.lerp(
                &self.target_tx.isometry.translation.vector,
                self.hot_focus
                    .map(|hf| self.foci[hf].weight_against_subject.clamp(0.0, 1.0))
                    .unwrap_or(0.0),
            ),
        ));
        self.world_tx
            .append_rotation_wrt_center_mut(&self.target_tx.isometry.rotation);

        self.screen_tx = self.world_tx.inverse();
    }

    /// The calculated "world transform" which maps from screen space to world space.
    pub fn screen_to_world_tx(&self) -> &Similarity2<f32> {
        &self.world_tx
    }

    /// The calculated "screen transform" which maps from world space to screen space, also known as
    /// the "view" transform of the Model View Projection approach to structuring graphics
    /// transforms.
    pub fn world_to_screen_tx(&self) -> &Similarity2<f32> {
        &self.screen_tx
    }

    pub fn view_tx(&self) -> Matrix4<f32> {
        homogeneous_mat3_to_mat4(&self.screen_tx.to_homogeneous())
    }
}

impl LuaUserData for Camera {}
