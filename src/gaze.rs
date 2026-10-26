//! Eye-contact geometry and native-model integration.
use crate::framing::Rect;
use anyhow::{Result, bail};

/// Maps continuous output pixel coordinates to source pixel coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Affine(pub [f32; 6]);
impl Affine {
    pub fn map(self, [x, y]: [f32; 2]) -> [f32; 2] {
        let [a, b, c, d, e, f] = self.0;
        [a * x + b * y + c, d * x + e * y + f]
    }
    pub fn inverse(self) -> Result<Self> {
        let [a, b, c, d, e, f] = self.0;
        let determinant = a * e - b * d;
        if self.0.iter().any(|v| !v.is_finite())
            || !determinant.is_finite()
            || determinant.abs() < 1e-8
        {
            bail!("Geometria dos olhos inválida");
        }
        Ok(Self([
            e / determinant,
            -b / determinant,
            (b * f - e * c) / determinant,
            -d / determinant,
            a / determinant,
            (d * c - a * f) / determinant,
        ]))
    }
    pub fn normalized(self, [w, h]: [usize; 2]) -> [f32; 6] {
        let [a, b, c, d, e, f] = self.0;
        [
            2.0 * a / w as f32,
            2.0 * b / w as f32,
            2.0 * c / w as f32 - 1.0,
            2.0 * d / h as f32,
            2.0 * e / h as f32,
            2.0 * f / h as f32 - 1.0,
        ]
    }
}

pub fn landmark_crop(face: Rect, [w, h]: [usize; 2]) -> Result<Affine> {
    if w == 0
        || h == 0
        || w > 7680
        || h > 4320
        || [face.x, face.y, face.width, face.height]
            .iter()
            .any(|v| !v.is_finite())
        || face.width <= 0.0
        || face.height <= 0.0
    {
        bail!("Geometria dos olhos inválida");
    }
    let side = (face.width * w as f32).max(face.height * h as f32) * 1.5;
    Ok(Affine([
        side / 80.0,
        0.0,
        (face.x + face.width * 0.5) * w as f32 - side * 0.5,
        0.0,
        side / 80.0,
        (face.y + face.height * 0.5) * h as f32 - side * 0.5,
    ]))
}

#[derive(Clone, Debug)]
pub struct Geometry {
    pub to_source: Affine,
    pub to_patch: Affine,
    /// Patch center and radii for each eye, in 256x64 pixel coordinates.
    pub eyes: [[f32; 4]; 2],
    pub visibility: [f32; 2],
}

fn smoothstep(low: f32, high: f32, value: f32) -> f32 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Stabilize alignment, never previous image pixels. Follow intentional movement
/// faster than subpixel landmark noise; a closed eye must bypass the fade at once.
pub struct Temporal {
    previous: Option<Geometry>,
    dt: f32,
}

impl Temporal {
    pub fn new(fps: f64) -> Result<Self> {
        if !fps.is_finite() || !(1.0..=60.0).contains(&fps) {
            bail!("FPS de processamento da IA inválido");
        }
        Ok(Self {
            previous: None,
            dt: 1.0 / fps as f32,
        })
    }

    pub fn reset(&mut self) {
        self.previous = None;
    }

    pub fn update(&mut self, mut current: Geometry) -> Result<Geometry> {
        let previous = self.previous.as_ref();
        if let Some(previous) = previous {
            let span = current.to_source.0[0].hypot(current.to_source.0[3]) * 128.0;
            let displacement = [[64.0, 32.0], [192.0, 32.0]]
                .into_iter()
                .map(|p| {
                    let a = current.to_source.map(p);
                    let b = previous.to_source.map(p);
                    (a[0] - b[0]).hypot(a[1] - b[1])
                })
                .fold(0.0_f32, f32::max);
            let motion = displacement / span.max(1.0);
            // A discontinuity is a new acquisition, not a path to interpolate.
            if motion < 0.35 {
                let moving = smoothstep(0.6, 3.0, motion / self.dt);
                let alpha = 1.0 - (-self.dt / (0.10 - moving * 0.092)).exp();
                for (value, old) in current.to_source.0.iter_mut().zip(previous.to_source.0) {
                    *value = old + alpha * (*value - old);
                }
                current.to_patch = current.to_source.inverse()?;
                let mask_alpha = 1.0 - (-self.dt / 0.08).exp();
                for (eye, old) in current.eyes.iter_mut().zip(previous.eyes) {
                    for i in 2..4 {
                        eye[i] = old[i] + mask_alpha * (eye[i] - old[i]);
                    }
                }
            } else {
                self.reset();
            }
        }
        for (i, visibility) in current.visibility.iter_mut().enumerate() {
            let old = self.previous.as_ref().map_or(0.0, |p| p.visibility[i]);
            if *visibility > 0.0 {
                let tau = if *visibility > old { 0.08 } else { 0.025 };
                *visibility = old + (1.0 - (-self.dt / tau).exp()) * (*visibility - old);
            }
        }
        self.previous = Some(current.clone());
        Ok(current)
    }
}

pub fn geometry(
    crop: Affine,
    points: &[f32],
    confidence: &[f32],
    pose: &[f32],
    [w, h]: [usize; 2],
) -> Result<Option<Geometry>> {
    if points.len() != 252
        || confidence.len() != 126
        || pose.len() != 3
        || points.iter().chain(pose).any(|v| !v.is_finite())
        || confidence
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        bail!("Saída de landmarks dos olhos inválida");
    }
    let pose_max = pose.iter().map(|v| v.abs()).fold(0.0_f32, f32::max);
    if pose_max > 0.45 {
        return Ok(None);
    }
    // These fixed indices are part of the validated 126-point tensor contract.
    let indices = [[64, 65, 67, 68, 69, 71], [81, 82, 84, 85, 86, 88]];
    let mut eyes = [[[0.0; 2]; 6]; 2];
    let mut reliability = [0.0; 2];
    for ((eye, reliability), indices) in eyes.iter_mut().zip(&mut reliability).zip(indices) {
        let mean = indices.iter().map(|&i| confidence[i]).sum::<f32>() / 6.0;
        if indices.iter().any(|&i| confidence[i] < 0.30) || mean < 0.45 {
            return Ok(None);
        }
        *reliability = smoothstep(0.45, 0.65, mean) * (1.0 - smoothstep(0.20, 0.45, pose_max));
        for (point, i) in eye.iter_mut().zip(indices) {
            *point = crop.map([points[i], points[126 + i]]);
        }
    }
    let center =
        |eye: &[[f32; 2]; 6]| [(eye[0][0] + eye[3][0]) * 0.5, (eye[0][1] + eye[3][1]) * 0.5];
    let left = center(&eyes[0]);
    let right = center(&eyes[1]);
    let delta = [right[0] - left[0], right[1] - left[1]];
    let distance = delta[0].hypot(delta[1]);
    if distance < 16.0 || distance > w as f32 * 0.5 || delta[0] <= 0.0 {
        return Ok(None);
    }
    let ux = delta[0] / 128.0;
    let uy = delta[1] / 128.0;
    let origin = [
        left[0] - 64.0 * ux + 32.0 * uy,
        left[1] - 64.0 * uy - 32.0 * ux,
    ];
    let to_source = Affine([ux, -uy, origin[0], uy, ux, origin[1]]);
    let to_patch = to_source.inverse()?;
    let mut bounds = [[0.0; 4]; 2];
    let mut visibility = [0.0; 2];
    for (i, eye) in eyes.iter().enumerate() {
        let eye = eye.map(|p| to_patch.map(p));
        let width = (eye[3][0] - eye[0][0]).abs();
        let opening = ((eye[1][1] - eye[5][1]).abs() + (eye[2][1] - eye[4][1]).abs()) * 0.5;
        if !(20.0..=88.0).contains(&width) {
            return Ok(None);
        }
        visibility[i] = smoothstep(0.13, 0.23, opening / width) * reliability[i];
        bounds[i] = [
            if i == 0 { 64.0 } else { 192.0 },
            32.0,
            (width * 0.55).clamp(11.0, 44.0),
            (opening * 0.85).max(width * 0.12).clamp(4.0, 16.0),
        ];
        let [cx, cy, rx, ry] = bounds[i];
        for p in [[cx - rx, cy - ry], [cx + rx, cy + ry]] {
            let [x, y] = to_source.map(p);
            if x < 1.0 || x > w as f32 - 1.0 || y < 1.0 || y > h as f32 - 1.0 {
                return Ok(None);
            }
        }
    }
    Ok(Some(Geometry {
        to_source,
        to_patch,
        eyes: bounds,
        visibility,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn eyes() -> Vec<f32> {
        let mut points = vec![0.0; 252];
        for (indices, cx) in [
            ([64, 65, 67, 68, 69, 71], 30.0),
            ([81, 82, 84, 85, 86, 88], 55.0),
        ] {
            for (i, [dx, dy]) in indices.into_iter().zip([
                [-5.0, 0.0],
                [-2.5, -3.0],
                [2.5, -3.0],
                [5.0, 0.0],
                [2.5, 3.0],
                [-2.5, 3.0],
            ]) {
                points[i] = cx + dx;
                points[126 + i] = 40.0 + dy;
            }
        }
        points
    }
    #[test]
    fn affine_alignment_inverts_rotation_and_rejects_degenerate_values() {
        let matrix = Affine([0.3, -0.1, 17.0, 0.1, 0.3, 80.0]);
        let inverse = matrix.inverse().unwrap();
        for point in [[0.0, 0.0], [64.0, 32.0], [256.0, 64.0]] {
            let actual = inverse.map(matrix.map(point));
            for i in 0..2 {
                assert!((actual[i] - point[i]).abs() < 0.0001);
            }
        }
        for value in [0.0, f32::NAN, f32::INFINITY, f32::MAX] {
            assert!(
                Affine([value, 0.0, 0.0, 0.0, value, 0.0])
                    .inverse()
                    .is_err()
            );
        }
        for size in [[0, 10], [8000, 10], [10, 4400]] {
            assert!(landmark_crop(Rect::FULL, size).is_err());
        }
    }
    #[test]
    fn eye_geometry_gates_low_confidence_head_pose_and_blinks() {
        let transform = Affine([2.0, 0.0, 0.0, 0.0, 2.0, 0.0]);
        let points = eyes();
        let mut confidence = vec![1.0; 126];
        let size = [200, 200];
        let valid = geometry(transform, &points, &confidence, &[0.0; 3], size)
            .unwrap()
            .unwrap();
        assert_eq!(valid.visibility, [1.0, 1.0]);
        for (canonical, expected) in [([64.0, 32.0], [60.0, 80.0]), ([192.0, 32.0], [110.0, 80.0])]
        {
            let actual = valid.to_source.map(canonical);
            for i in 0..2 {
                assert!((actual[i] - expected[i]).abs() < 1e-5);
            }
        }
        assert!(
            geometry(transform, &points, &confidence, &[0.5, 0.0, 0.0], size)
                .unwrap()
                .is_none()
        );
        confidence.fill(0.1);
        assert!(
            geometry(transform, &points, &confidence, &[0.0; 3], size)
                .unwrap()
                .is_none()
        );
        for value in [-1.0, 1.1, f32::NAN, f32::INFINITY] {
            confidence[0] = value;
            assert!(geometry(transform, &points, &confidence, &[0.0; 3], size).is_err());
        }
        confidence.fill(1.0);
        let mut closed = points.clone();
        for i in [65, 67, 69, 71] {
            closed[126 + i] = 40.0;
        }
        assert_eq!(
            geometry(transform, &closed, &confidence, &[0.0; 3], size)
                .unwrap()
                .unwrap()
                .visibility,
            [0.0, 1.0]
        );
        for i in [82, 84, 86, 88] {
            closed[126 + i] = 40.0;
        }
        assert!(
            geometry(transform, &closed, &confidence, &[0.0; 3], size)
                .unwrap()
                .is_some_and(|g| g.visibility == [0.0, 0.0])
        );
        assert!(geometry(transform, &points[..251], &confidence, &[0.0; 3], size).is_err());
        closed[64] = f32::NAN;
        assert!(geometry(transform, &closed, &confidence, &[0.0; 3], size).is_err());
    }

    fn frontal() -> Geometry {
        let to_source = Affine([0.5, 0.0, 100.0, 0.0, 0.5, 100.0]);
        Geometry {
            to_source,
            to_patch: to_source.inverse().unwrap(),
            eyes: [[64.0, 32.0, 25.0, 10.0], [192.0, 32.0, 25.0, 10.0]],
            visibility: [1.0, 1.0],
        }
    }

    #[test]
    fn temporal_reduces_jitter_follows_motion_and_never_delays_a_blink() {
        for fps in [15.0, 30.0, 60.0] {
            let mut temporal = Temporal::new(fps).unwrap();
            for _ in 0..60 {
                temporal.update(frontal()).unwrap();
            }
            let mut previous = 100.0;
            let mut variation = 0.0;
            for i in 0..120 {
                let mut measured = frontal();
                measured.to_source.0[2] += if i % 2 == 0 { 0.75 } else { -0.75 };
                let filtered = temporal.update(measured).unwrap();
                variation += (filtered.to_source.0[2] - previous).abs();
                previous = filtered.to_source.0[2];
            }
            assert!(variation < 120.0 * 1.5 * 0.35, "{fps} fps: {variation}");
            let mut moved = frontal();
            moved.to_source.0[2] += 12.0;
            let followed = temporal.update(moved.clone()).unwrap();
            assert!(
                (followed.to_source.0[2] - 112.0).abs() < 2.0,
                "{fps} fps: {followed:?}"
            );
            moved.visibility = [0.0, 1.0];
            let blink = temporal.update(moved.clone()).unwrap();
            assert_eq!(blink.visibility[0], 0.0);
            assert!(blink.visibility[1] > 0.99);
            moved.visibility = [1.0, 1.0];
            let reopen = temporal.update(moved).unwrap();
            assert!(reopen.visibility[0] > 0.0 && reopen.visibility[0] < 0.6);
            assert!(reopen.visibility[1] > 0.99);
            temporal.reset();
            let fresh = temporal.update(frontal()).unwrap();
            assert!(fresh.visibility[0] < 0.6);
            let mut different = frontal();
            different.to_source.0[2] += 100.0;
            let acquired = temporal.update(different).unwrap();
            assert_eq!(acquired.to_source.0[2], 200.0);
            assert!(
                acquired.visibility[0] < 0.6,
                "a new face must not inherit the previous gain"
            );
        }
        for fps in [0.0, -1.0, 61.0, f64::NAN, f64::INFINITY] {
            assert!(Temporal::new(fps).is_err());
        }
    }

    #[test]
    fn visibility_fades_before_pose_confidence_and_blink_cutoffs() {
        let transform = Affine([2.0, 0.0, 0.0, 0.0, 2.0, 0.0]);
        let points = eyes();
        let estimate = |confidence, pose| {
            geometry(
                transform,
                &points,
                &vec![confidence; 126],
                &[pose, 0.0, 0.0],
                [200, 200],
            )
            .unwrap()
            .unwrap()
            .visibility[0]
        };
        assert!(estimate(0.45, 0.0) < 1e-10);
        assert!(estimate(0.451, 0.0) < 0.001);
        assert!(estimate(0.55, 0.0) > 0.0 && estimate(0.55, 0.0) < 1.0);
        assert_eq!(estimate(1.0, 0.45), 0.0);
        assert!(estimate(1.0, 0.449) < 0.001);
    }
}
