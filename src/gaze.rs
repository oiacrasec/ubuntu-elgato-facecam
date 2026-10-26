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
    pub open: [bool; 2],
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
    if pose.iter().any(|v| v.abs() > 0.45) {
        return Ok(None);
    }
    // These fixed indices are part of the validated 126-point tensor contract.
    let indices = [[64, 65, 67, 68, 69, 71], [81, 82, 84, 85, 86, 88]];
    let mut eyes = [[[0.0; 2]; 6]; 2];
    for (eye, indices) in eyes.iter_mut().zip(indices) {
        if indices.iter().any(|&i| confidence[i] < 0.30)
            || indices.iter().map(|&i| confidence[i]).sum::<f32>() / 6.0 < 0.45
        {
            return Ok(None);
        }
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
    let mut open = [false; 2];
    for (i, eye) in eyes.iter().enumerate() {
        let eye = eye.map(|p| to_patch.map(p));
        let width = (eye[3][0] - eye[0][0]).abs();
        let opening = ((eye[1][1] - eye[5][1]).abs() + (eye[2][1] - eye[4][1]).abs()) * 0.5;
        if !(20.0..=88.0).contains(&width) {
            return Ok(None);
        }
        open[i] = opening / width > 0.13;
        bounds[i] = [
            if i == 0 { 64.0 } else { 192.0 },
            32.0,
            (width * 0.70).clamp(14.0, 44.0),
            (opening * 1.3).max(width * 0.22).clamp(6.0, 20.0),
        ];
        let [cx, cy, rx, ry] = bounds[i];
        for p in [[cx - rx, cy - ry], [cx + rx, cy + ry]] {
            let [x, y] = to_source.map(p);
            if x < 1.0 || x > w as f32 - 1.0 || y < 1.0 || y > h as f32 - 1.0 {
                return Ok(None);
            }
        }
    }
    if !open.iter().any(|v| *v) {
        return Ok(None);
    }
    Ok(Some(Geometry {
        to_source,
        to_patch,
        eyes: bounds,
        open,
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
        assert_eq!(valid.open, [true, true]);
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
                .open,
            [false, true]
        );
        for i in [82, 84, 86, 88] {
            closed[126 + i] = 40.0;
        }
        assert!(
            geometry(transform, &closed, &confidence, &[0.0; 3], size)
                .unwrap()
                .is_none()
        );
        assert!(geometry(transform, &points[..251], &confidence, &[0.0; 3], size).is_err());
        closed[64] = f32::NAN;
        assert!(geometry(transform, &closed, &confidence, &[0.0; 3], size).is_err());
    }
}
