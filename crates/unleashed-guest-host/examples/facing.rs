//! Does the body Sonic is rendered with face the way he moves? Prints the angle between the
//! body's forward axis (+Z rotated by the body quaternion) and the velocity on the ground.
use unleashed_guest_host::*;

fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let [x, y, z, w] = q;
    let u = [x, y, z];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let t = cross(u, v).map(|c| 2.0 * c);
    let c2 = cross(u, t);
    [v[0] + w * t[0] + c2[0], v[1] + w * t[1] + c2[1], v[2] + w * t[2] + c2[2]]
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    for f in 0..240u32 {
        let stick = if f < 120 { [0.0, 1.0] } else { [1.0, 0.2] };
        let s = g.step(&Pad { left: stick, ..Default::default() });
        if f % 30 == 29 {
            let fwd = rotate(s.body_rotation, [0.0, 0.0, 1.0]);
            let v = s.velocity;
            let (a, b) = ((fwd[0], fwd[2]), (v[0], v[2]));
            let ang = (a.0 * b.1 - a.1 * b.0).atan2(a.0 * b.0 + a.1 * b.1).to_degrees();
            println!("f{f:3} speed {:6.1}  body fwd {:5.2?}  vel dir {:5.2?}  angle {ang:7.1} deg",
                (v[0] * v[0] + v[2] * v[2]).sqrt(), [fwd[0], fwd[2]], {
                    let l = (v[0] * v[0] + v[2] * v[2]).sqrt().max(1e-6);
                    [v[0] / l, v[2] / l]
                });
        }
    }
}
