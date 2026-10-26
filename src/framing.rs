//! Small face-detection tensors used internally by eye-contact processing.
use anyhow::{Result, bail};

pub const WIDTH: usize = 368;
pub const HEIGHT: usize = 208;
const COLS: usize = 23;
const ROWS: usize = 13;
const CELLS: usize = COLS * ROWS;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rect {
    #[cfg(test)]
    pub const FULL: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
    pub(crate) fn overlap(self, other: Self) -> f32 {
        let w = (self.x + self.width).min(other.x + other.width) - self.x.max(other.x);
        let h = (self.y + self.height).min(other.y + other.height) - self.y.max(other.y);
        let area = w.max(0.0) * h.max(0.0);
        area / (self.width * self.height + other.width * other.height - area)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Letterbox {
    pub size: [usize; 2],
    pub offset: [usize; 2],
}
impl Letterbox {
    pub fn new([w, h]: [usize; 2]) -> Result<Self> {
        if w == 0 || h == 0 || w > 7680 || h > 4320 {
            bail!("Resolução IA fora dos limites");
        }
        let scale = (WIDTH as f64 / w as f64).min(HEIGHT as f64 / h as f64);
        let rw = ((w as f64 * scale).round() as usize).clamp(1, WIDTH);
        let rh = ((h as f64 * scale).round() as usize).clamp(1, HEIGHT);
        Ok(Self {
            size: [rw, rh],
            offset: [(WIDTH - rw) / 2, (HEIGHT - rh) / 2],
        })
    }

    pub fn decode(self, boxes: &[f32], confidence: &[f32]) -> Result<Vec<Rect>> {
        if boxes.len() != 4 * CELLS
            || confidence.len() != CELLS
            || boxes.iter().any(|v| !v.is_finite())
            || confidence
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            bail!("Saída de detecção de rosto inválida");
        }
        let mut candidates = Vec::new();
        for (i, &score) in confidence.iter().enumerate().filter(|(_, s)| **s >= 0.7) {
            // The validated Ada model predicts distances from a stride-16 grid,
            // scaled by 35 pixels. Decode only its small output tensors on CPU.
            let cx = (i % COLS) as f32 * 16.0 + 0.5;
            let cy = (i / COLS) as f32 * 16.0 + 0.5;
            let x0 = ((cx - 35.0 * boxes[i] - self.offset[0] as f32) / self.size[0] as f32)
                .clamp(0.0, 1.0);
            let y0 = ((cy - 35.0 * boxes[CELLS + i] - self.offset[1] as f32) / self.size[1] as f32)
                .clamp(0.0, 1.0);
            let x1 = ((cx + 35.0 * boxes[2 * CELLS + i] - self.offset[0] as f32)
                / self.size[0] as f32)
                .clamp(0.0, 1.0);
            let y1 = ((cy + 35.0 * boxes[3 * CELLS + i] - self.offset[1] as f32)
                / self.size[1] as f32)
                .clamp(0.0, 1.0);
            if x1 > x0 && y1 > y0 {
                candidates.push((
                    score,
                    Rect {
                        x: x0,
                        y: y0,
                        width: x1 - x0,
                        height: y1 - y0,
                    },
                ));
            }
        }
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut faces: Vec<Rect> = Vec::new();
        for (_, rect) in candidates {
            if faces.iter().all(|face| face.overlap(rect) < 0.3) {
                faces.push(rect);
            }
            if faces.len() == 32 {
                break;
            }
        }
        Ok(faces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detection_decodes_grid_distances_unpads_clips_and_suppresses_duplicates() {
        let letterbox = Letterbox::new([1920, 1080]).unwrap();
        assert_eq!(letterbox.size, [368, 207]);
        let mut boxes = vec![0.0; 4 * CELLS];
        let mut confidence = vec![0.0; CELLS];
        for (i, score) in [(5 * COLS + 10, 0.95), (5 * COLS + 11, 0.8)] {
            let cx = (i % COLS) as f32 * 16.0 + 0.5;
            let cy = (i / COLS) as f32 * 16.0 + 0.5;
            for (plane, v) in [
                (0, (cx - 120.0) / 35.0),
                (1, (cy - 30.0) / 35.0),
                (2, (220.0 - cx) / 35.0),
                (3, (140.0 - cy) / 35.0),
            ] {
                boxes[plane * CELLS + i] = v;
            }
            confidence[i] = score;
        }
        let faces = letterbox.decode(&boxes, &confidence).unwrap();
        assert_eq!(faces.len(), 1);
        assert!((faces[0].x - 120.0 / 368.0).abs() < 1e-6);
        assert!((faces[0].height - 110.0 / 207.0).abs() < 1e-6);
        assert!(letterbox.decode(&boxes[..10], &confidence).is_err());
        confidence[0] = f32::NAN;
        assert!(letterbox.decode(&boxes, &confidence).is_err());
        for value in [-0.1, 1.1, f32::INFINITY] {
            confidence[0] = value;
            assert!(letterbox.decode(&boxes, &confidence).is_err());
        }
        confidence.fill(0.0);
        confidence[0] = 0.7;
        for value in [-1.0, 0.0] {
            boxes.fill(value);
            assert!(letterbox.decode(&boxes, &confidence).unwrap().is_empty());
        }
        boxes.fill(f32::MAX);
        assert_eq!(
            letterbox.decode(&boxes, &confidence).unwrap(),
            vec![Rect::FULL]
        );
        boxes[0] = f32::NAN;
        assert!(letterbox.decode(&boxes, &confidence).is_err());
        assert!(Letterbox::new([0, 10]).is_err());
        assert_eq!(Letterbox::new([1080, 1920]).unwrap().size, [117, 208]);
    }
}
