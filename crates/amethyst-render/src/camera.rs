use amethyst_math::prelude::*;

/// A simple camera that can be used to view a scene.
pub struct Camera {
    projection: glm::Mat4,
    direction: glm::Vec3,
    position: glm::Vec3,
}

impl Camera {
    #[must_use]
    pub fn new(info: CameraCreateInfo) -> Self {
        let mut projection = glm::perspective_rh_zo(
            info.width / info.height,
            glm::radians(&glm::vec1(info.fov))[0],
            info.near,
            info.far,
        );

        // The Y axis of the projection matrix is inverted because Vulkan
        // uses a different coordinate system than OpenGL
        projection[(1, 1)] *= -1.0;

        Self {
            direction: info.direction,
            position: info.position,
            projection,
        }
    }

    /// Sets the direction that the camera is looking at.
    #[must_use]
    pub fn set_direction(&mut self, direction: glm::Vec3) {
        self.direction = direction;
    }

    /// Sets the position of the camera.
    #[must_use]
    pub fn set_position(&mut self, position: glm::Vec3) {
        self.position = position;
    }

    /// Returns the direction that the camera is looking at.
    #[must_use]
    pub fn direction(&self) -> &glm::Vec3 {
        &self.direction
    }

    /// Returns the position of the camera.
    #[must_use]
    pub fn position(&self) -> &glm::Vec3 {
        &self.position
    }

    /// Returns the projection matrix of the camera.
    #[must_use]
    pub fn projection(&self) -> &glm::Mat4 {
        &self.projection
    }

    /// Returns the view matrix of the camera. The matrix is calculated
    /// every time this function is called to account for the camera
    /// moving.
    #[must_use]
    pub fn view(&self) -> glm::Mat4 {
        glm::look_at(&self.position, &self.direction, &glm::vec3(0.0, 0.0, 1.0))
    }
}

pub struct CameraCreateInfo {
    /// The height of the camera, in pixels.
    pub height: f32,

    /// The width of the camera, in pixels.
    pub width: f32,

    /// The field of view of the camera, in degrees.
    pub fov: f32,

    /// The near clipping plane of the camera.
    pub near: f32,

    /// The far clipping plane of the camera.
    pub far: f32,

    /// The position of the camera.
    pub position: glm::Vec3,

    /// The position that the camera is looking at.
    pub direction: glm::Vec3,
}

impl Default for CameraCreateInfo {
    fn default() -> Self {
        Self {
            height: 600.0,
            width: 800.0,
            fov: 90.0,
            near: 0.1,
            far: 100.0,
            position: glm::vec3(0.0, 0.0, 0.0),
            direction: glm::vec3(0.0, 0.0, 1.0),
        }
    }
}
