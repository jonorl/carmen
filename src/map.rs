//! The map shown when departing: Natural Earth land outlines, rasterised
//! into a pixel-art texture for whatever area the view covers.

use macroquad::prelude::*;

/// Land polygons as rings of (lon, lat) points.
pub struct Land {
    rings: Vec<Vec<(f32, f32)>>,
}

impl Land {
    pub fn load() -> Land {
        let geo: serde_json::Value =
            serde_json::from_str(include_str!("../assets/ne_110m_land.geojson")).unwrap_or_default();
        let point = |p: &serde_json::Value| Some((p.get(0)?.as_f64()? as f32, p.get(1)?.as_f64()? as f32));
        let mut rings = Vec::new();
        for feature in geo["features"].as_array().into_iter().flatten() {
            let geometry = &feature["geometry"];
            let polygons: Vec<&serde_json::Value> = match geometry["type"].as_str() {
                Some("Polygon") => vec![&geometry["coordinates"]],
                Some("MultiPolygon") => geometry["coordinates"].as_array().into_iter().flatten().collect(),
                _ => continue,
            };
            for ring in polygons.into_iter().filter_map(|p| p.as_array()).flatten() {
                rings.push(ring.as_array().into_iter().flatten().filter_map(point).collect());
            }
        }
        Land { rings }
    }
}

const OCEAN: [u8; 3] = [0x00, 0x00, 0xaa];
const GRID: [u8; 3] = [0x22, 0x22, 0xcc];
const LAND: [u8; 3] = [0x00, 0xaa, 0x00];
const COAST: [u8; 3] = [0x55, 0xff, 0x55];

/// An equirectangular view: longitude is squeezed by cos(latitude) at the
/// view's centre so shapes look about right at any zoom.
#[derive(Clone, Copy)]
pub struct MapView {
    pub rect: Rect,
    lon0: f32,
    lat0: f32,
    kx: f32,
    /// Pixels per degree of latitude.
    ppd: f32,
}

impl MapView {
    /// A view that fits all the given (lat, lon) points with a margin.
    pub fn fit(rect: Rect, points: &[(f64, f64)]) -> MapView {
        let mean = points.iter().map(|p| p.0 as f32).sum::<f32>() / points.len().max(1) as f32;
        let kx = mean.clamp(-60.0, 60.0).to_radians().cos();
        let xs = points.iter().map(|p| p.1 as f32 * kx);
        let ys = points.iter().map(|p| p.0 as f32);
        let (x0, x1) = xs.fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
        let (y0, y1) = ys.fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
        let span_x = (x1 - x0).max(30.0) * 1.35;
        let span_y = (y1 - y0).max(30.0) * 1.35;
        // Never zoom out further than the whole world fits across.
        let ppd = (rect.w / span_x).min(rect.h / span_y).max(rect.w / (360.0 * kx));
        MapView { rect, lon0: (x0 + x1) / 2.0 / kx, lat0: (y0 + y1) / 2.0, kx, ppd }
    }

    pub fn project(&self, lat: f64, lon: f64) -> Vec2 {
        vec2(
            self.rect.x + self.rect.w / 2.0 + (lon as f32 - self.lon0) * self.kx * self.ppd,
            self.rect.y + self.rect.h / 2.0 - (lat as f32 - self.lat0) * self.ppd,
        )
    }

    /// Paint the land into a texture the size of the view, by scanline
    /// filling the polygons (even-odd rule), then mark the coast.
    pub fn render(&self, land: &Land) -> Texture2D {
        let (w, h) = (self.rect.w as usize, self.rect.h as usize);
        let origin = vec2(self.rect.x, self.rect.y);
        let mut edges = Vec::new();
        for ring in &land.rings {
            let pts: Vec<Vec2> = ring.iter().map(|&(lon, lat)| self.project(lat as f64, lon as f64) - origin).collect();
            edges.extend(pts.windows(2).map(|p| (p[0], p[1])));
        }
        let mut is_land = vec![false; w * h];
        let mut crossings = Vec::new();
        for py in 0..h {
            let yc = py as f32 + 0.5;
            crossings.clear();
            for &(a, b) in &edges {
                if (a.y <= yc) != (b.y <= yc) {
                    crossings.push(a.x + (yc - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
            crossings.sort_by(f32::total_cmp);
            for pair in crossings.chunks_exact(2) {
                let from = (pair[0] - 0.5).ceil().max(0.0) as usize;
                let to = ((pair[1] - 0.5).ceil().max(0.0) as usize).min(w);
                for px in from..to {
                    is_land[py * w + px] = true;
                }
            }
        }

        // Dotted lines every 30 degrees of latitude and longitude.
        let step = 30.0;
        let lon_at = |px: usize| self.lon0 + (px as f32 + 0.5 - w as f32 / 2.0) / (self.kx * self.ppd);
        let lat_at = |py: usize| self.lat0 - (py as f32 + 0.5 - h as f32 / 2.0) / self.ppd;
        let near_line = |deg: f32, per_px: f32| (deg / step - (deg / step).round()).abs() * step < per_px / 2.0;
        let (lon_px, lat_px) = (1.0 / (self.kx * self.ppd), 1.0 / self.ppd);

        let mut bytes = Vec::with_capacity(w * h * 4);
        for py in 0..h {
            for px in 0..w {
                let here = is_land[py * w + px];
                let colour = if here {
                    let neighbours = [(px.wrapping_sub(1), py), (px + 1, py), (px, py.wrapping_sub(1)), (px, py + 1)];
                    let coast = neighbours.iter().any(|&(x, y)| x < w && y < h && !is_land[y * w + x]);
                    if coast { COAST } else { LAND }
                } else if (px + py) % 2 == 0 && (near_line(lon_at(px), lon_px) || near_line(lat_at(py), lat_px)) {
                    GRID
                } else {
                    OCEAN
                };
                bytes.extend([colour[0], colour[1], colour[2], 255]);
            }
        }
        let texture = Texture2D::from_image(&Image { bytes, width: w as u16, height: h as u16 });
        texture.set_filter(FilterMode::Nearest);
        texture
    }
}

/// The part of the segment a-b inside r (Liang-Barsky clipping).
pub fn clip(a: Vec2, b: Vec2, r: Rect) -> Option<(Vec2, Vec2)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [(-d.x, a.x - r.x), (d.x, r.x + r.w - a.x), (-d.y, a.y - r.y), (d.y, r.y + r.h - a.y)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 <= t1).then(|| (a + d * t0, a + d * t1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_segments_to_a_rectangle() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(clip(vec2(-5.0, 5.0), vec2(5.0, 5.0), r), Some((vec2(0.0, 5.0), vec2(5.0, 5.0))));
        assert_eq!(clip(vec2(-5.0, -5.0), vec2(-1.0, -1.0), r), None);
    }

    #[test]
    fn a_fitted_view_contains_its_points() {
        let r = Rect::new(0.0, 0.0, 270.0, 280.0);
        let points = [(27.7, 85.3), (-12.0, -77.0), (51.5, -0.1)];
        let view = MapView::fit(r, &points);
        for (lat, lon) in points {
            assert!(r.contains(view.project(lat, lon)), "{lat},{lon} outside");
        }
    }
}
